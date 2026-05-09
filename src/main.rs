use chrono::Local;
use serde::Deserialize;
use std::env;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Write as _;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const VERSION: &str = "v0.8.0";

#[derive(Debug, Deserialize)]
struct Profile {
    name: String,
    domains: Vec<String>,
    tcp_test_domain: String,
}

#[derive(Debug, Clone, Default)]
struct NetworkAdapterInfo {
    name: String,
    description: String,
    status: String,
    dhcp_enabled: String,
    ipv4_address: String,
    default_gateway: String,
    dns_servers: Vec<String>,
}

#[derive(Debug)]
struct ReportSummary {
    profiles_tested: usize,
    dns_failures: usize,
    tcp_failures: usize,
    warnings: usize,
    reasons: Vec<String>,
}

impl ReportSummary {
    fn overall_status(&self) -> &'static str {
        if self.dns_failures > 0 || self.tcp_failures > 0 {
            "PROBLEM"
        } else if self.warnings > 0 {
            "WARNING"
        } else {
            "OK"
        }
    }

    fn add_warning(&mut self, reason: &str) {
        self.warnings += 1;
        self.reasons.push(reason.to_string());
    }

    fn add_dns_failure(&mut self, domain: &str) {
        self.dns_failures += 1;
        self.reasons.push(format!("DNS failed for {}.", domain));
    }

    fn add_tcp_failure(&mut self, domain: &str) {
        self.tcp_failures += 1;
        self.reasons.push(format!("TCP 443 failed for {}.", domain));
    }
}

#[derive(Debug)]
struct ParsedReportSummary {
    profiles_tested: usize,
    dns_failures: usize,
    tcp_failures: usize,
    warnings: usize,
    overall_status: String,
    reasons: Vec<String>,
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
        "profiles" => profiles(),
        "validate" => validate_profiles(),
        "restore" => restore(),
        "report" => report(),
        "netinfo" => netinfo(),
        "compare" => {
            if args.len() < 4 {
                println!("Usage: lag compare <old_report> <new_report>");
                println!("Example: lag compare reports\\old.txt reports\\new.txt");
                return;
            }

            compare_reports(&args[2], &args[3]);
        }
        _ => print_help(),
    }
}

fn print_help() {
    println!("LocalAccessGuard {}", VERSION);
    println!();
    println!("Commands:");
    println!("  status");
    println!("  doctor <profile>");
    println!("  profiles");
    println!("  validate");
    println!("  restore");
    println!("  report");
    println!("  netinfo");
    println!("  compare <old_report> <new_report>");
    println!();
    println!("Examples:");
    println!("  doctor discord");
    println!("  doctor roblox");
    println!("  compare reports\\old.txt reports\\new.txt");
}

fn profiles() {
    let entries = match profile_files() {
        Ok(entries) => entries,
        Err(_) => {
            println!("No valid profiles found.");
            return;
        }
    };

    let mut valid_profiles = Vec::new();

    for path in entries {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let profile = match parse_and_validate_profile(&text) {
            Ok(profile) => profile,
            Err(_) => continue,
        };

        let key = match profile_key_from_path(&path) {
            Some(key) => key,
            None => continue,
        };

        valid_profiles.push((key, profile.name));
    }

    if valid_profiles.is_empty() {
        println!("No valid profiles found.");
        return;
    }

    valid_profiles.sort_by(|left, right| left.0.cmp(&right.0));

    println!("Available profiles:");
    for (key, name) in valid_profiles {
        println!("- {}: {}", key, name);
    }
}

fn validate_profiles() {
    let entries = match profile_files() {
        Ok(entries) => entries,
        Err(err) => {
            println!("Could not read profiles directory: {}", err);
            return;
        }
    };

    println!("Profile validation:");

    for path in entries {
        let display_path = path.to_string_lossy();

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                println!("{}: INVALID - could not read file: {}", display_path, err);
                continue;
            }
        };

        match parse_and_validate_profile(&text) {
            Ok(_) => println!("{}: OK", display_path),
            Err(err) => println!("{}: INVALID - {}", display_path, err),
        }
    }
}

fn status() {
    println!("=== LocalAccessGuard Status ===");
    println!();

    check_windows_proxy();
    check_autoconfig_url();
    check_winhttp_proxy();
    check_processes();
}

fn netinfo() {
    println!("=== Network Info ===");
    println!();

    let adapters = get_network_adapters();
    print_network_adapters(&adapters);
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

fn compare_reports(old_report_path: &str, new_report_path: &str) {
    let old_text = match fs::read_to_string(old_report_path) {
        Ok(text) => text,
        Err(err) => {
            println!("Failed to read old report '{}': {}", old_report_path, err);
            return;
        }
    };

    let new_text = match fs::read_to_string(new_report_path) {
        Ok(text) => text,
        Err(err) => {
            println!("Failed to read new report '{}': {}", new_report_path, err);
            return;
        }
    };

    let old_summary = match parse_report_summary(&old_text) {
        Ok(summary) => summary,
        Err(err) => {
            println!("Invalid old report '{}': {}", old_report_path, err);
            return;
        }
    };

    let new_summary = match parse_report_summary(&new_text) {
        Ok(summary) => summary,
        Err(err) => {
            println!("Invalid new report '{}': {}", new_report_path, err);
            return;
        }
    };

    let removed_reasons = diff_reasons(&old_summary.reasons, &new_summary.reasons);
    let added_reasons = diff_reasons(&new_summary.reasons, &old_summary.reasons);
    let summary_changed = old_summary.profiles_tested != new_summary.profiles_tested
        || old_summary.dns_failures != new_summary.dns_failures
        || old_summary.tcp_failures != new_summary.tcp_failures
        || old_summary.warnings != new_summary.warnings
        || old_summary.overall_status != new_summary.overall_status;
    let reasons_changed = !removed_reasons.is_empty() || !added_reasons.is_empty();

    println!("Report Compare");
    println!();
    println!("Old report: {}", old_report_path);
    println!("New report: {}", new_report_path);
    println!();

    if !summary_changed && !reasons_changed {
        println!("No summary changes detected.");
        return;
    }

    println!("[Summary Changes]");
    println!(
        "Profiles tested: {} -> {}",
        old_summary.profiles_tested, new_summary.profiles_tested
    );
    println!(
        "DNS failures: {} -> {}",
        old_summary.dns_failures, new_summary.dns_failures
    );
    println!(
        "TCP failures: {} -> {}",
        old_summary.tcp_failures, new_summary.tcp_failures
    );
    println!(
        "Warnings: {} -> {}",
        old_summary.warnings, new_summary.warnings
    );
    println!(
        "Overall status: {} -> {}",
        old_summary.overall_status, new_summary.overall_status
    );
    println!();
    println!("[Reason Changes]");
    print_reason_list("Removed:", &removed_reasons);
    println!();
    print_reason_list("Added:", &added_reasons);
}

fn parse_report_summary(report_text: &str) -> Result<ParsedReportSummary, String> {
    let mut in_summary = false;
    let mut summary_lines = Vec::new();

    for line in report_text.lines() {
        let trimmed = line.trim();

        if trimmed == "[Summary]" {
            in_summary = true;
            continue;
        }

        if in_summary && trimmed.starts_with('[') && trimmed.ends_with(']') {
            break;
        }

        if in_summary {
            summary_lines.push(trimmed.to_string());
        }
    }

    if !in_summary {
        return Err("missing [Summary] section".to_string());
    }

    let profiles_tested = parse_summary_usize(&summary_lines, "Profiles tested")?;
    let dns_failures = parse_summary_usize(&summary_lines, "DNS failures")?;
    let tcp_failures = parse_summary_usize(&summary_lines, "TCP failures")?;
    let warnings = parse_summary_usize(&summary_lines, "Warnings")?;
    let overall_status = parse_summary_string(&summary_lines, "Overall status")?;
    let reasons = parse_summary_reasons(&summary_lines)?;

    Ok(ParsedReportSummary {
        profiles_tested,
        dns_failures,
        tcp_failures,
        warnings,
        overall_status,
        reasons,
    })
}

fn parse_summary_usize(lines: &[String], field: &str) -> Result<usize, String> {
    let value = parse_summary_string(lines, field)?;

    value
        .parse::<usize>()
        .map_err(|_| format!("invalid {} value: {}", field, value))
}

fn parse_summary_string(lines: &[String], field: &str) -> Result<String, String> {
    let prefix = format!("{}:", field);

    for line in lines {
        if let Some(value) = line.strip_prefix(&prefix) {
            let value = value.trim();

            if value.is_empty() {
                return Err(format!("missing {} value", field));
            }

            return Ok(value.to_string());
        }
    }

    Err(format!("missing required field: {}", field))
}

fn parse_summary_reasons(lines: &[String]) -> Result<Vec<String>, String> {
    let reasons_index = lines
        .iter()
        .position(|line| line == "Reasons:")
        .ok_or_else(|| "missing required field: Reasons".to_string())?;
    let mut reasons = Vec::new();

    for line in lines.iter().skip(reasons_index + 1) {
        if line.is_empty() {
            break;
        }

        let Some(reason) = line.strip_prefix("- ") else {
            return Err("invalid Reasons list entry".to_string());
        };

        let reason = reason.trim();

        if reason.eq_ignore_ascii_case("none.") || reason.eq_ignore_ascii_case("none") {
            continue;
        }

        if reason.is_empty() {
            return Err("invalid empty Reasons list entry".to_string());
        }

        reasons.push(reason.to_string());
    }

    Ok(reasons)
}

fn diff_reasons(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .filter(|reason| !right.contains(reason))
        .cloned()
        .collect()
}

fn print_reason_list(label: &str, reasons: &[String]) {
    println!("{}", label);

    if reasons.is_empty() {
        println!("- None");
        return;
    }

    for reason in reasons {
        println!("- {}", reason);
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

    parse_and_validate_profile(&text)
}

fn load_all_profiles() -> Vec<Profile> {
    let mut profiles = Vec::new();

    let entries = match profile_files() {
        Ok(entries) => entries,
        Err(_) => return profiles,
    };

    for path in entries {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let profile = match parse_and_validate_profile(&text) {
            Ok(profile) => profile,
            Err(_) => continue,
        };

        profiles.push(profile);
    }

    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    profiles
}

fn profile_files() -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir("profiles").map_err(|err| err.to_string())?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();

        let is_json = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("json"))
            .unwrap_or(false);

        if is_json {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn profile_key_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
}

fn parse_and_validate_profile(text: &str) -> Result<Profile, String> {
    let profile: Profile =
        serde_json::from_str(text).map_err(|err| format!("invalid JSON: {}", err))?;

    validate_profile(&profile)?;

    Ok(profile)
}

fn validate_profile(profile: &Profile) -> Result<(), String> {
    if profile.name.trim().is_empty() {
        return Err("missing or empty name".to_string());
    }

    if profile.domains.is_empty() {
        return Err("missing or empty domains".to_string());
    }

    if profile
        .domains
        .iter()
        .any(|domain| domain.trim().is_empty())
    {
        return Err("domains contains empty entry".to_string());
    }

    if profile.tcp_test_domain.trim().is_empty() {
        return Err("missing or empty tcp_test_domain".to_string());
    }

    Ok(())
}

fn build_report_text() -> String {
    let mut report = String::new();
    let summary = build_report_summary();

    let _ = writeln!(report, "LocalAccessGuard Report");
    let _ = writeln!(report, "Version: {}", VERSION);
    let _ = writeln!(
        report,
        "Generated: {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let _ = writeln!(report);
    append_report_summary(&mut report, &summary);

    append_windows_proxy_report(&mut report);
    append_autoconfig_report(&mut report);
    append_winhttp_report(&mut report);
    append_network_info_report(&mut report);
    append_process_report(&mut report);
    append_profiles_report(&mut report);

    report
}

fn build_report_summary() -> ReportSummary {
    let mut summary = ReportSummary {
        profiles_tested: 0,
        dns_failures: 0,
        tcp_failures: 0,
        warnings: 0,
        reasons: Vec::new(),
    };

    let proxy_enable = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyEnable",
    );

    if proxy_enable
        .as_deref()
        .map(|text| text.contains("0x1"))
        .unwrap_or(false)
    {
        summary.add_warning("Windows proxy is currently active.");
    }

    let proxy_server = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyServer",
    );

    if proxy_server
        .as_deref()
        .map(|text| text.contains("127.0.0.1") || text.contains("localhost"))
        .unwrap_or(false)
    {
        summary.add_warning("Stale local proxy entry exists.");
    }

    if read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "AutoConfigURL",
    )
    .is_some()
    {
        summary.add_warning("AutoConfigURL / PAC proxy config exists.");
    }

    add_process_warnings(&mut summary);
    add_profile_results(&mut summary);

    summary
}

fn add_process_warnings(summary: &mut ReportSummary) {
    let known_warning_processes = [
        "warp-svc.exe",
        "goodbyedpi.exe",
        "bypax-proxy.exe",
        "BypaxDPI.exe",
    ];
    let output = Command::new("tasklist").output();

    let text = match output {
        Ok(result) => String::from_utf8_lossy(&result.stdout).to_lowercase(),
        Err(_) => return,
    };

    for process in known_warning_processes {
        let process_lower = process.to_lowercase();

        if !text.contains(&process_lower) {
            continue;
        }

        if process == "warp-svc.exe" {
            summary.add_warning("Cloudflare WARP service is running in the background.");
        } else {
            summary.add_warning(&format!("DPI/proxy tool process is active: {}", process));
        }
    }
}

fn add_profile_results(summary: &mut ReportSummary) {
    let profiles = load_all_profiles();
    summary.profiles_tested = profiles.len();

    if profiles.is_empty() {
        summary.add_warning("No valid profiles found.");
        return;
    }

    for profile in profiles {
        for domain in &profile.domains {
            let dns_result = get_dns_result(domain);

            if !is_ok_result(&dns_result) {
                summary.add_dns_failure(domain);
            }
        }

        let tcp_result = get_tcp_443_result(&profile.tcp_test_domain);

        if !is_ok_result(&tcp_result) {
            summary.add_tcp_failure(&profile.tcp_test_domain);
        }
    }
}

fn append_report_summary(report: &mut String, summary: &ReportSummary) {
    let _ = writeln!(report, "[Summary]");
    let _ = writeln!(report, "Profiles tested: {}", summary.profiles_tested);
    let _ = writeln!(report, "DNS failures: {}", summary.dns_failures);
    let _ = writeln!(report, "TCP failures: {}", summary.tcp_failures);
    let _ = writeln!(report, "Warnings: {}", summary.warnings);
    let _ = writeln!(report, "Overall status: {}", summary.overall_status());
    let _ = writeln!(report, "Reasons:");

    if summary.reasons.is_empty() {
        let _ = writeln!(report, "- None.");
    } else {
        for reason in &summary.reasons {
            let _ = writeln!(report, "- {}", reason);
        }
    }

    let _ = writeln!(report);
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

fn append_network_info_report(report: &mut String) {
    let _ = writeln!(report, "[Network Info]");

    let adapters = get_network_adapters();

    if adapters.is_empty() {
        let _ = writeln!(report, "Adapter Name: Unknown");
        let _ = writeln!(report, "Description: Unknown");
        let _ = writeln!(report, "DHCP Enabled: Unknown");
        let _ = writeln!(report, "IPv4 Address: Unknown");
        let _ = writeln!(report, "Default Gateway: Unknown");
        let _ = writeln!(report, "DNS Servers:");
        let _ = writeln!(report, "- Unknown");
        let _ = writeln!(report);
        return;
    }

    for (index, adapter) in adapters.iter().enumerate() {
        if index > 0 {
            let _ = writeln!(report);
        }

        let _ = writeln!(report, "Adapter Name: {}", known_or_unknown(&adapter.name));
        let _ = writeln!(
            report,
            "Description: {}",
            known_or_unknown(&adapter.description)
        );
        let _ = writeln!(
            report,
            "DHCP Enabled: {}",
            known_or_unknown(&adapter.dhcp_enabled)
        );
        let _ = writeln!(
            report,
            "IPv4 Address: {}",
            known_or_unknown(&adapter.ipv4_address)
        );
        let _ = writeln!(
            report,
            "Default Gateway: {}",
            known_or_unknown(&adapter.default_gateway)
        );
        let _ = writeln!(report, "DNS Servers:");

        if adapter.dns_servers.is_empty() {
            let _ = writeln!(report, "- Unknown");
        } else {
            for dns_server in &adapter.dns_servers {
                let _ = writeln!(report, "- {}", dns_server);
            }
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
                        let _ = writeln!(
                            report,
                            "Warning: DPI/proxy tool process is active: {}",
                            process
                        );
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

fn is_ok_result(result: &str) -> bool {
    result.starts_with("OK")
}

fn get_network_adapters() -> Vec<NetworkAdapterInfo> {
    let output = Command::new("ipconfig").arg("/all").output();

    let text = match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);

            if !stdout.trim().is_empty() {
                stdout.to_string()
            } else {
                stderr.to_string()
            }
        }
        Err(_) => return Vec::new(),
    };

    let adapters = parse_ipconfig_adapters(&text);
    let active_adapters: Vec<NetworkAdapterInfo> = adapters
        .iter()
        .filter(|adapter| !adapter.ipv4_address.is_empty() && !adapter.default_gateway.is_empty())
        .cloned()
        .collect();

    if active_adapters.is_empty() {
        adapters
            .into_iter()
            .filter(|adapter| !adapter.ipv4_address.is_empty())
            .collect()
    } else {
        active_adapters
    }
}

fn parse_ipconfig_adapters(text: &str) -> Vec<NetworkAdapterInfo> {
    let mut adapters = Vec::new();
    let mut current: Option<NetworkAdapterInfo> = None;
    let mut reading_dns_servers = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            reading_dns_servers = false;
            continue;
        }

        if is_adapter_heading(line) {
            if let Some(adapter) = current.take() {
                adapters.push(adapter);
            }

            current = Some(NetworkAdapterInfo {
                name: parse_adapter_name(trimmed),
                status: "Up".to_string(),
                ..NetworkAdapterInfo::default()
            });
            reading_dns_servers = false;
            continue;
        }

        let Some(adapter) = current.as_mut() else {
            continue;
        };

        if reading_dns_servers && is_ip_address_like(trimmed) {
            adapter.dns_servers.push(clean_ip_value(trimmed));
            continue;
        }

        let Some((label, value)) = parse_ipconfig_field(trimmed) else {
            reading_dns_servers = false;
            continue;
        };

        if is_description_label(&label) {
            adapter.description = value;
            reading_dns_servers = false;
        } else if is_dhcp_label(&label) {
            adapter.dhcp_enabled = normalize_yes_no(&value);
            reading_dns_servers = false;
        } else if is_ipv4_label(&label) {
            adapter.ipv4_address = clean_ip_value(&value);
            reading_dns_servers = false;
        } else if is_gateway_label(&label) {
            adapter.default_gateway = clean_ip_value(&value);
            reading_dns_servers = false;
        } else if is_dns_label(&label) {
            if !value.is_empty() {
                adapter.dns_servers.push(clean_ip_value(&value));
            }
            reading_dns_servers = true;
        } else {
            reading_dns_servers = false;
        }
    }

    if let Some(adapter) = current {
        adapters.push(adapter);
    }

    adapters
}

fn print_network_adapters(adapters: &[NetworkAdapterInfo]) {
    if adapters.is_empty() {
        println!("[Adapter]");
        println!("Name: Unknown");
        println!("Description: Unknown");
        println!("Status: Unknown");
        println!("DHCP Enabled: Unknown");
        println!("IPv4 Address: Unknown");
        println!("Default Gateway: Unknown");
        println!("DNS Servers:");
        println!("- Unknown");
        return;
    }

    for (index, adapter) in adapters.iter().enumerate() {
        if index > 0 {
            println!();
        }

        println!("[Adapter]");
        println!("Name: {}", known_or_unknown(&adapter.name));
        println!("Description: {}", known_or_unknown(&adapter.description));
        println!("Status: {}", known_or_unknown(&adapter.status));
        println!("DHCP Enabled: {}", known_or_unknown(&adapter.dhcp_enabled));
        println!("IPv4 Address: {}", known_or_unknown(&adapter.ipv4_address));
        println!(
            "Default Gateway: {}",
            known_or_unknown(&adapter.default_gateway)
        );
        println!("DNS Servers:");

        if adapter.dns_servers.is_empty() {
            println!("- Unknown");
        } else {
            for dns_server in &adapter.dns_servers {
                println!("- {}", dns_server);
            }
        }
    }
}

fn is_adapter_heading(line: &str) -> bool {
    let trimmed = line.trim();

    if !trimmed.ends_with(':') || trimmed.contains(". :") || trimmed.contains(" .") {
        return false;
    }

    let normalized = normalize_label(trimmed);

    normalized.contains("adapter")
        || normalized.contains("bagdastir")
        || normalized.contains("badatr")
        || normalized.contains("badt")
}

fn parse_adapter_name(heading: &str) -> String {
    let heading = heading.trim_end_matches(':').trim();
    let mut parts = heading.rsplitn(2, ' ');

    parts
        .next()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| heading.to_string())
}

fn parse_ipconfig_field(line: &str) -> Option<(String, String)> {
    let (label, value) = line.split_once(':')?;

    Some((normalize_label(label), value.trim().to_string()))
}

fn is_description_label(label: &str) -> bool {
    label.contains("description") || label.contains("aciklama") || label.contains("klama")
}

fn is_dhcp_label(label: &str) -> bool {
    label.contains("dhcp") && (label.contains("enabled") || label.contains("etkin"))
}

fn is_ipv4_label(label: &str) -> bool {
    label.contains("ipv4")
}

fn is_gateway_label(label: &str) -> bool {
    label.contains("default gateway")
        || label.contains("varsay")
        || label.contains("ag gecidi")
        || label.contains("gecidi")
}

fn is_dns_label(label: &str) -> bool {
    label.contains("dns") && (label.contains("server") || label.contains("sunucu"))
}

fn normalize_label(label: &str) -> String {
    label
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | ' ' => character,
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'İ' | 'ı' => 'i',
            'Ö' | 'ö' => 'o',
            'Ş' | 'ş' => 's',
            'Ü' | 'ü' => 'u',
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_ip_value(value: &str) -> String {
    let before_marker = value
        .split_once('(')
        .map(|(left, _)| left)
        .unwrap_or(value)
        .trim();

    before_marker
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(',')
        .trim()
        .to_string()
}

fn is_ip_address_like(value: &str) -> bool {
    let value = clean_ip_value(value);
    value.chars().any(|character| character.is_ascii_digit())
        && (value.contains('.') || value.contains(':'))
}

fn normalize_yes_no(value: &str) -> String {
    let normalized = normalize_label(value);

    if normalized == "yes" || normalized == "evet" {
        "Yes".to_string()
    } else if normalized == "no" || normalized == "hayir" {
        "No".to_string()
    } else {
        value.trim().to_string()
    }
}

fn known_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "Unknown"
    } else {
        value
    }
}
