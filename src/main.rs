use std::env;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;

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
                println!("Usage: lag doctor <discord|roblox>");
                return;
            }

            match args[2].as_str() {
                "discord" => doctor_discord(),
                "roblox" => doctor_roblox(),
                _ => println!("Unknown profile: {}", args[2]),
            }
        }
        "restore" => restore(),
        "report" => report(),
        _ => print_help(),
    }
}

fn print_help() {
    println!("LocalAccessGuard v0.2.0");
    println!();
    println!("Commands:");
    println!("  status");
    println!("  doctor discord");
    println!("  doctor roblox");
    println!("  restore");
    println!("  report");
}

fn status() {
    println!("=== LocalAccessGuard Status ===");
    println!();

    check_windows_proxy();
    check_autoconfig_url();
    check_winhttp_proxy();
    check_processes();
}

fn doctor_discord() {
    println!("=== Doctor: Discord ===");
    println!();

    check_domain("discord.com");
    check_domain("discord.gg");
    check_tcp_443("discord.com");
    check_processes();
}

fn doctor_roblox() {
    println!("=== Doctor: Roblox ===");
    println!();

    check_domain("roblox.com");
    check_domain("www.roblox.com");
    check_tcp_443("roblox.com");
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
    println!("Report function is not implemented yet.");
    println!("v0.3 goal: export diagnostic report to a text file.");
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
                        println!("    Warning: Cloudflare WARP service is running in the background.");
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