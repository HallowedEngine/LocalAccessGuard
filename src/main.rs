use std::env;
use std::process::Command;
use std::net::{TcpStream, ToSocketAddrs};
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
    println!("LocalAccessGuard v0.1");
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
    println!("Restore function is not implemented yet.");
    println!("v0.2 will restore proxy / WinHTTP / DNS settings safely.");
}

fn report() {
    println!("Report function is not implemented yet.");
    println!("v0.1 goal: print system network status.");
}

fn check_windows_proxy() {
    println!("[Windows Proxy]");

    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyEnable",
        ])
        .output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout);
            if text.contains("0x1") {
                println!("  ProxyEnable: Enabled");
            } else if text.contains("0x0") {
                println!("  ProxyEnable: Disabled");
            } else {
                println!("  ProxyEnable: Unknown");
            }
        }
        Err(err) => {
            println!("  Error reading proxy status: {}", err);
        }
    }

    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyServer",
        ])
        .output();

    match output {
        Ok(result) => {
            let text = String::from_utf8_lossy(&result.stdout);
            if text.trim().is_empty() {
                println!("  ProxyServer: Not set");
            } else {
                println!("  ProxyServer:");
                println!("{}", text);
            }
        }
        Err(_) => {
            println!("  ProxyServer: Not found");
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