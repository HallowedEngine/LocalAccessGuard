use crate::config::Config;
use std::process::Command;

fn check_windows_proxy() {
    println!("[Windows Proxy]");

    let proxy_enable = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyEnable",
    );

    match proxy_enable {
        Some(text) => {
            if text.contains("0x1") {
                println!("  [WARNING] ProxyEnable: Enabled");
                println!("  [WARNING] Windows proxy is currently active.");
            } else if text.contains("0x0") {
                println!("  [OK] ProxyEnable: Disabled");
            } else {
                println!("  [INFO] ProxyEnable: Unknown");
                println!("{}", text.trim());
            }
        }
        None => {
            println!("  [WARNING] ProxyEnable: Not found");
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
                println!("  [WARNING] Stale local proxy entry exists.");
            }
        }
        None => {
            println!("  [OK] ProxyServer: Not set");
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
            println!("  [WARNING] PAC proxy config exists.");
        }
        None => {
            println!("  [OK] AutoConfigURL: Not set");
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
            let trimmed = text.trim();

            if is_winhttp_direct_access(trimmed) {
                println!("[OK] {}", trimmed);
            } else {
                println!("[INFO] {}", trimmed);
            }
        }
        Err(err) => {
            println!("  [FAILED] Error reading WinHTTP proxy: {}", err);
        }
    }

    println!();
}

pub(crate) fn check_processes(config: &Config) {
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
                    println!("  [INFO] {}: Running", process);

                    if process == "warp-svc.exe" && config.show_warp_warning {
                        println!(
                            "    [WARNING] Cloudflare WARP service is running in the background."
                        );
                    }

                    if process == "goodbyedpi.exe"
                        || process == "bypax-proxy.exe"
                        || process == "BypaxDPI.exe"
                    {
                        println!("    [WARNING] DPI/proxy tool process is active.");
                    }
                } else {
                    println!("  [OK] {}: Not running", process);
                }
            }
        }
        Err(err) => {
            println!("  [FAILED] Error reading process list: {}", err);
        }
    }

    println!();
}

pub(crate) fn read_reg_value(path: &str, value_name: &str) -> Option<String> {
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
        println!("[OK] SKIP - already clean");
        return;
    }

    let output = Command::new("reg")
        .args(["delete", path, "/v", value_name, "/f"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("[OK] OK");
            } else {
                println!("[FAILED] FAILED");

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
            println!("[FAILED] ERROR: {}", err);
        }
    }
}

fn run_command(program: &str, args: &[&str], label: &str) {
    print!("{}... ", label);

    let output = Command::new(program).args(args).output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("[OK] OK");
            } else {
                println!("[FAILED] FAILED");

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
            println!("[FAILED] ERROR: {}", err);
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

            if is_winhttp_direct_access(&text) {
                println!("[OK] SKIP - already clean");
                return;
            }
        }
        Err(err) => {
            println!(
                "[FAILED] ERROR while checking current WinHTTP proxy: {}",
                err
            );
            return;
        }
    }

    let reset_output = Command::new("netsh")
        .args(["winhttp", "reset", "proxy"])
        .output();

    match reset_output {
        Ok(result) => {
            if result.status.success() {
                println!("[OK] OK");
            } else {
                println!("[FAILED] FAILED - admin permission may be required");

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
            println!("[FAILED] ERROR: {}", err);
        }
    }
}

fn is_winhttp_direct_access(text: &str) -> bool {
    text.contains("Direct access") || text.contains("DoÄŸrudan eriÅŸim")
}

pub fn status(config: &Config) {
    println!("=== LocalAccessGuard Status ===");
    println!();

    check_windows_proxy();
    check_autoconfig_url();
    check_winhttp_proxy();
    check_processes(config);
}

pub fn restore() {
    println!("=== LocalAccessGuard Restore ===");
    println!();

    println!("[INFO] This will disable Windows user proxy and clear stale proxy entries.");
    println!("[INFO] It will also reset WinHTTP proxy.");
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
    println!("[OK] Restore completed.");
    println!("[INFO] Run `cargo run -- status` again to verify.");
}
