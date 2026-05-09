use chrono::Local;
use serde::Deserialize;
use std::env;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Write as _;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Profile {
    name: String,
    domains: Vec<String>,
    tcp_test_domain: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "status" => status(),
        "doctor" => {
            if args.len() < 3 {
                println!("Usage: lag doctor <profile>");
                println!("Example: lag doctor discord");
                println!("Example: lag doctor roblox");
                return;
            }

            doctor_profile(&args[2]);
        }
        "restore" => restore(),
        "report" => report(),
        _ => print_help(),
    }
}

fn print_help() {
    println!("LocalAccessGuard v0.4.0");
    println!();
    println!("Commands:");
    println!("  status");
    println!("  doctor <profile>");
    println!("  restore");
    println!("  report");
    println!();
    println!("Examples:");
    println!("  doctor discord");
    println!("  doctor roblox");
}

fn status() {
    println!("=== LocalAccessGuard Status ===");
    println!();

    check_windows_proxy();
    check_autoconfig_url();
    check_winhttp_proxy();
    check_processes();
}

fn doctor_profile(profile_name: &str) {
    let profile = match load_profile(profile_name) {
        Ok(profile) => profile,
        Err(err) => {
            println!("Failed to load profile '{}': {}", profile_name, err);
            println!("Expected file: profiles\\{}.json", profile_name);
            return;
        }
    };

    println!("=== Doctor: {} ===", profile.name);
    println!();

    for domain in &profile.domains {
        check_domain(domain);
    }

    check_tcp_443(&profile.tcp_test_domain);
    check_processes();
}

fn restore() {
    println!("=== LocalAccessGuard Restore ===");
    println!();

    println!("This will disable Windows user proxy and clear stale proxy entries.");
    println!("It will also reset WinHTTP proxy.");
    println!();

    run_command(
        "reg",
        &[
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyEnable",
            "/t",
            "REG_DWORD",
            "/d",
            "0",
            "/f",
        ],
        "Disable Windows Proxy",
    );

    delete_registry_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyServer",
        "Delete stale ProxyServer",
    );

    delete_registry_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "AutoConfigURL",
        "Delete AutoConfigURL / PAC proxy",
    );

    reset_winhttp_proxy();

    println!();
    println!("Restore completed.");
    println!("Run `cargo run -- status` again to verify.");
}

fn report() {
    println!("=== LocalAccessGuard Report ===");
    println!();

    let report_text = build_report_text();

    match fs::create_dir_all("reports") {
        Ok(_) => {}
        Err(err) => {
            println!("Failed to create reports directory: {}", err);
            return;
        }
    }

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let file_path = format!("reports\\local_access_report_{}.txt", timestamp);

    let file_result = File::create(&file_path);

    let mut file = match file_result {
        Ok(file) => file,
        Err(err) => {
            println!("Failed to create report file: {}", err);
            return;
        }
    };

    match file.write_all(report_text.as_bytes()) {
        Ok(_) => {
            println!("Report saved:");
            println!("{}", file_path);
        }
        Err(err) => {
            println!("Failed to write report file: {}", err);
        }
    }
}

fn check_windows_proxy() {
    println!("[Windows Proxy]");

    let proxy_enable = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyEnable",
    );

    match proxy_enable {
        Some(text) => {
            if text.contains("0x1") {
                println!("  ProxyEnable: Enabled");
                println!("  Warning: Windows proxy is currently active.");
            } else if text.contains("0x0") {
                println!("  ProxyEnable: Disabled");
            } else {
                println!("  ProxyEnable: Unknown");
                println!("{}", text.trim());
            }
        }
        None => {
            println!("  ProxyEnable: Not found");
        }
    }

    let proxy_server = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyServer",
    );

    match proxy_server {
        Some(text) => {
            println!("  ProxyServer:");
            println!("{}", text.trim());

            if text.contains("127.0.0.1") || text.contains("localhost") {
                println!("  Warning: Stale local proxy entry exists.");
            }
        }
        None => {
            println!("  ProxyServer: Not set");
        }
    }

    println!();
}

fn check_autoconfig_url() {
    println!("[Auto Proxy / PAC]");

    let autoconfig = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "AutoConfigURL",
    );

    match autoconfig {
        Some(text) => {
            println!("  AutoConfigURL:");
            println!("{}", text.trim());
            println!("  Warning: PAC proxy config exists.");
        }
        None => {
            println!("  AutoConfigURL: Not set");
        }
    }

    println!();
}

fn check_winhttp_proxy() {
    println!("[WinHTTP Proxy]");

    let output = Command::new("netsh")
        .args(["winhttp", "show", "proxy"])
        .output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout);
            println!("{}", text.trim());
        }
        Err(err) => {
            println!("  Error reading WinHTTP proxy: {}", err);
        }
    }

    println!();
}

fn check_processes() {
    println!("[Known Network Tools]");

    let known_processes = [
        "warp-svc.exe",
        "Cloudflare WARP.exe",
        "goodbyedpi.exe",
        "bypax-proxy.exe",
        "BypaxDPI.exe",
        "Discord.exe",
        "RobloxPlayerBeta.exe",
    ];

    let output = Command::new("tasklist").output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout).to_lowercase();

            for process in known_processes {
                let process_lower = process.to_lowercase();

                if text.contains(&process_lower) {
                    println!("  {}: Running", process);

                    if process == "warp-svc.exe" {
                        println!(
                            "    Warning: Cloudflare WARP service is running in the background."
                        );
                    }

                    if process == "goodbyedpi.exe"
                        || process == "bypax-proxy.exe"
                        || process == "BypaxDPI.exe"
                    {
                        println!("    Warning: DPI/proxy tool process is active.");
                    }
                } else {
                    println!("  {}: Not running", process);
                }
            }
        }
        Err(err) => {
            println!("  Error reading process list: {}", err);
        }
    }

    println!();
}

fn check_domain(domain: &str) {
    println!("[DNS Test: {}]", domain);

    let address = format!("{}:443", domain);

    match address.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                println!("  Resolved: {}", addr.ip());
            } else {
                println!("  Failed: no address returned");
            }
        }
        Err(err) => {
            println!("  Failed: {}", err);
        }
    }

    println!();
}

fn check_tcp_443(domain: &str) {
    println!("[TCP 443 Test: {}]", domain);

    let address = format!("{}:443", domain);

    match address.to_socket_addrs() {
        Ok(addrs) => {
            let mut success = false;

            for addr in addrs {
                let result = TcpStream::connect_timeout(&addr, Duration::from_secs(3));

                if result.is_ok() {
                    println!("  TCP 443: OK ({})", addr);
                    success = true;
                    break;
                }
            }

            if !success {
                println!("  TCP 443: Failed");
            }
        }
        Err(err) => {
            println!("  Could not resolve address: {}", err);
        }
    }

    println!();
}

fn read_reg_value(path: &str, value_name: &str) -> Option<String> {
    let output = Command::new("reg")
        .args(["query", path, "/v", value_name])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let text = String::from_utf8_lossy(&result.stdout).to_string();

                if text.trim().is_empty() {
                    None
                } else {
                    Some(text)
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

fn delete_registry_value(path: &str, value_name: &str, label: &str) {
    print!("{}... ", label);

    let existing = read_reg_value(path, value_name);

    if existing.is_none() {
        println!("SKIP - already clean");
        return;
    }

    let output = Command::new("reg")
        .args(["delete", path, "/v", value_name, "/f"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("OK");
            } else {
                println!("FAILED");

                let stderr = String::from_utf8_lossy(&result.stderr);
                let stdout = String::from_utf8_lossy(&result.stdout);

                if !stdout.trim().is_empty() {
                    println!("  stdout: {}", stdout.trim());
                }

                if !stderr.trim().is_empty() {
                    println!("  stderr: {}", stderr.trim());
                }
            }
        }
        Err(err) => {
            println!("ERROR: {}", err);
        }
    }
}

fn run_command(program: &str, args: &[&str], label: &str) {
    print!("{}... ", label);

    let output = Command::new(program).args(args).output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("OK");
            } else {
                println!("FAILED");

                let stderr = String::from_utf8_lossy(&result.stderr);
                let stdout = String::from_utf8_lossy(&result.stdout);

                if !stdout.trim().is_empty() {
                    println!("  stdout: {}", stdout.trim());
                }

                if !stderr.trim().is_empty() {
                    println!("  stderr: {}", stderr.trim());
                }
            }
        }
        Err(err) => {
            println!("ERROR: {}", err);
        }
    }
}

fn reset_winhttp_proxy() {
    print!("Reset WinHTTP proxy... ");

    let output = Command::new("netsh")
        .args(["winhttp", "show", "proxy"])
        .output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout);

            if text.contains("Direct access") || text.contains("Doğrudan erişim") {
                println!("SKIP - already clean");
                return;
            }
        }
        Err(err) => {
            println!("ERROR while checking current WinHTTP proxy: {}", err);
            return;
        }
    }

    let reset_output = Command::new("netsh")
        .args(["winhttp", "reset", "proxy"])
        .output();

    match reset_output {
        Ok(result) => {
            if result.status.success() {
                println!("OK");
            } else {
                println!("FAILED - admin permission may be required");

                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                if !stdout.trim().is_empty() {
                    println!("  stdout: {}", stdout.trim());
                }

                if !stderr.trim().is_empty() {
                    println!("  stderr: {}", stderr.trim());
                }
            }
        }
        Err(err) => {
            println!("ERROR: {}", err);
        }
    }
}

fn load_profile(profile_name: &str) -> Result<Profile, String> {
    let file_path = format!("profiles\\{}.json", profile_name);

    let text = fs::read_to_string(&file_path)
        .map_err(|err| format!("could not read {}: {}", file_path, err))?;

    let profile: Profile =
        serde_json::from_str(&text).map_err(|err| format!("invalid JSON: {}", err))?;

    if profile.name.trim().is_empty() {
        return Err("profile name is empty".to_string());
    }

    if profile.domains.is_empty() {
        return Err("profile domains list is empty".to_string());
    }

    if profile.tcp_test_domain.trim().is_empty() {
        return Err("tcp_test_domain is empty".to_string());
    }

    Ok(profile)
}

fn load_all_profiles() -> Vec<Profile> {
    let mut profiles = Vec::new();

    let entries = match fs::read_dir("profiles") {
        Ok(entries) => entries,
        Err(_) => return profiles,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let is_json = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("json"))
            .unwrap_or(false);

        if !is_json {
            continue;
        }

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let profile: Profile = match serde_json::from_str(&text) {
            Ok(profile) => profile,
            Err(_) => continue,
        };

        if profile.name.trim().is_empty()
            || profile.domains.is_empty()
            || profile.tcp_test_domain.trim().is_empty()
        {
            continue;
        }

        profiles.push(profile);
    }

    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    profiles
}

fn build_report_text() -> String {
    let mut report = String::new();

    let _ = writeln!(report, "LocalAccessGuard Report");
    let _ = writeln!(report, "Version: v0.4.0");
    let _ = writeln!(
        report,
        "Generated: {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let _ = writeln!(report);

    append_windows_proxy_report(&mut report);
    append_autoconfig_report(&mut report);
    append_winhttp_report(&mut report);
    append_process_report(&mut report);
    append_profiles_report(&mut report);

    report
}

fn append_windows_proxy_report(report: &mut String) {
    let _ = writeln!(report, "[Windows Proxy]");

    let proxy_enable = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyEnable",
    );

    match proxy_enable {
        Some(text) => {
            if text.contains("0x1") {
                let _ = writeln!(report, "ProxyEnable: Enabled");
                let _ = writeln!(report, "Warning: Windows proxy is currently active.");
            } else if text.contains("0x0") {
                let _ = writeln!(report, "ProxyEnable: Disabled");
            } else {
                let _ = writeln!(report, "ProxyEnable: Unknown");
                let _ = writeln!(report, "{}", text.trim());
            }
        }
        None => {
            let _ = writeln!(report, "ProxyEnable: Not found");
        }
    }

    let proxy_server = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyServer",
    );

    match proxy_server {
        Some(text) => {
            let _ = writeln!(report, "ProxyServer:");
            let _ = writeln!(report, "{}", text.trim());

            if text.contains("127.0.0.1") || text.contains("localhost") {
                let _ = writeln!(report, "Warning: Stale local proxy entry exists.");
            }
        }
        None => {
            let _ = writeln!(report, "ProxyServer: Not set");
        }
    }

    let _ = writeln!(report);
}

fn append_autoconfig_report(report: &mut String) {
    let _ = writeln!(report, "[Auto Proxy / PAC]");

    let autoconfig = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "AutoConfigURL",
    );

    match autoconfig {
        Some(text) => {
            let _ = writeln!(report, "AutoConfigURL:");
            let _ = writeln!(report, "{}", text.trim());
            let _ = writeln!(report, "Warning: PAC proxy config exists.");
        }
        None => {
            let _ = writeln!(report, "AutoConfigURL: Not set");
        }
    }

    let _ = writeln!(report);
}

fn append_winhttp_report(report: &mut String) {
    let _ = writeln!(report, "[WinHTTP Proxy]");

    let output = Command::new("netsh")
        .args(["winhttp", "show", "proxy"])
        .output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout);
            let _ = writeln!(report, "{}", text.trim());
        }
        Err(err) => {
            let _ = writeln!(report, "Error reading WinHTTP proxy: {}", err);
        }
    }

    let _ = writeln!(report);
}

fn append_process_report(report: &mut String) {
    let _ = writeln!(report, "[Known Network Tools]");

    let known_processes = [
        "warp-svc.exe",
        "Cloudflare WARP.exe",
        "goodbyedpi.exe",
        "bypax-proxy.exe",
        "BypaxDPI.exe",
        "Discord.exe",
        "RobloxPlayerBeta.exe",
    ];

    let output = Command::new("tasklist").output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout).to_lowercase();

            for process in known_processes {
                let process_lower = process.to_lowercase();

                if text.contains(&process_lower) {
                    let _ = writeln!(report, "{}: Running", process);

                    if process == "warp-svc.exe" {
                        let _ = writeln!(
                            report,
                            "Warning: Cloudflare WARP service is running in the background."
                        );
                    }

                    if process == "goodbyedpi.exe"
                        || process == "bypax-proxy.exe"
                        || process == "BypaxDPI.exe"
                    {
                        let _ = writeln!(report, "Warning: DPI/proxy tool process is active.");
                    }
                } else {
                    let _ = writeln!(report, "{}: Not running", process);
                }
            }
        }
        Err(err) => {
            let _ = writeln!(report, "Error reading process list: {}", err);
        }
    }

    let _ = writeln!(report);
}

fn append_profiles_report(report: &mut String) {
    let profiles = load_all_profiles();

    if profiles.is_empty() {
        let _ = writeln!(report, "[Profiles]");
        let _ = writeln!(report, "No valid profiles found in profiles\\*.json");
        let _ = writeln!(report);
        return;
    }

    for profile in profiles {
        append_service_report(
            report,
            &profile.name,
            &profile.domains,
            &profile.tcp_test_domain,
        );
    }
}

fn append_service_report(
    report: &mut String,
    service_name: &str,
    domains: &[String],
    tcp_test_domain: &str,
) {
    let _ = writeln!(report, "[{}]", service_name);

    for domain in domains {
        let dns_result = get_dns_result(domain);
        let _ = writeln!(report, "DNS {}: {}", domain, dns_result);
    }

    let tcp_result = get_tcp_443_result(tcp_test_domain);
    let _ = writeln!(report, "TCP 443 {}: {}", tcp_test_domain, tcp_result);

    let _ = writeln!(report);
}

fn get_dns_result(domain: &str) -> String {
    let address = format!("{}:443", domain);

    match address.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => format!("OK ({})", addr.ip()),
            None => "FAILED - no address returned".to_string(),
        },
        Err(err) => format!("FAILED - {}", err),
    }
}

fn get_tcp_443_result(domain: &str) -> String {
    let address = format!("{}:443", domain);

    match address.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                let result = TcpStream::connect_timeout(&addr, Duration::from_secs(3));

                if result.is_ok() {
                    return format!("OK ({})", addr);
                }
            }

            "FAILED".to_string()
        }
        Err(err) => format!("FAILED - could not resolve address: {}", err),
    }
}
