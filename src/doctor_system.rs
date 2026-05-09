use crate::config::Config;
use crate::firewall::inspect_firewall_targets;
use crate::logger;
use crate::network::{get_network_adapters, run_udp_diagnostics};
use crate::services::inspect_services;
use crate::windows::read_reg_value;
use std::process::Command;

pub fn doctor_system(config: &Config) {
    println!("=== System Doctor ===");
    println!();

    let mut issues = Vec::new();
    let mut suggestions = Vec::new();

    inspect_proxy_state(&mut issues, &mut suggestions);
    inspect_process_warnings(config, &mut issues, &mut suggestions);
    inspect_configured_profiles(config);
    inspect_udp(config, &mut issues);
    inspect_network_info();
    inspect_firewall_summary(config);
    inspect_services_summary(config, &mut issues, &mut suggestions);

    let overall_status = if issues.is_empty() { "OK" } else { "WARNING" };
    logger::info(
        config,
        &format!("doctor-system overall status={}", overall_status),
    );

    println!("[Summary]");
    println!("Overall status: {}", overall_status);
    println!();

    if issues.is_empty() {
        println!("[OK] No major system-level issues detected.");
    } else {
        println!("[Issues]");
        for issue in &issues {
            println!("- {}", issue);
        }
        println!();
    }

    println!("[Suggestions]");
    suggestions
        .push("Run `restore` only if proxy/PAC/WinHTTP settings are stale or broken.".to_string());
    suggestions.push("Generate a report with `report` before and after changes.".to_string());
    suggestions
        .push("Use `compare <old_report> <new_report>` to confirm what changed.".to_string());

    for suggestion in unique_strings(suggestions) {
        println!("- {}", suggestion);
    }
}

fn inspect_proxy_state(issues: &mut Vec<String>, suggestions: &mut Vec<String>) {
    println!("[Windows Proxy]");

    let proxy_enable = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "ProxyEnable",
    );

    match proxy_enable {
        Some(text) if text.contains("0x1") => {
            println!("ProxyEnable: Enabled");
            issues.push("Windows user proxy is enabled.".to_string());
            suggestions.push(
                "If the proxy entry is stale, use Windows Settings or `restore` to clear it."
                    .to_string(),
            );
        }
        Some(_) => println!("ProxyEnable: Disabled"),
        None => println!("ProxyEnable: Unknown"),
    }

    let autoconfig = read_reg_value(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        "AutoConfigURL",
    );

    if autoconfig.is_some() {
        println!("AutoConfigURL: Set");
        issues.push("Auto Proxy / PAC configuration is set.".to_string());
    } else {
        println!("AutoConfigURL: Not set");
    }

    let winhttp = Command::new("netsh")
        .args(["winhttp", "show", "proxy"])
        .output();

    match winhttp {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let trimmed = text.trim();

            if trimmed.contains("Direct access") {
                println!("WinHTTP: Direct access");
            } else if trimmed.is_empty() {
                println!("WinHTTP: Unknown");
            } else {
                println!("WinHTTP: {}", trimmed);
                issues.push("WinHTTP proxy may be configured.".to_string());
            }
        }
        Err(err) => println!("WinHTTP: Unknown ({})", err),
    }

    println!();
}

fn inspect_process_warnings(
    config: &Config,
    issues: &mut Vec<String>,
    suggestions: &mut Vec<String>,
) {
    println!("[Known Network Tools]");

    let output = Command::new("tasklist").output();
    let Ok(output) = output else {
        println!("[WARNING] Could not inspect process list.");
        return;
    };

    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    let watched = [
        "warp-svc.exe",
        "Cloudflare WARP.exe",
        "goodbyedpi.exe",
        "bypax-proxy.exe",
        "BypaxDPI.exe",
    ];

    for process in watched {
        if text.contains(&process.to_lowercase()) {
            println!("[INFO] {}: Running", process);

            if process == "warp-svc.exe" && config.show_warp_warning {
                issues.push("Cloudflare WARP service is running in the background.".to_string());
                suggestions.push(
                    "If you are not using Cloudflare WARP, close or disable it from the official app."
                        .to_string(),
                );
            }

            if process == "goodbyedpi.exe"
                || process == "bypax-proxy.exe"
                || process == "BypaxDPI.exe"
            {
                issues.push(format!("Network tool process is active: {}", process));
                suggestions.push(
                    "Close third-party network tools from their own app if they are interfering."
                        .to_string(),
                );
            }
        } else {
            println!("[OK] {}: Not running", process);
        }
    }

    println!();
}

fn inspect_configured_profiles(config: &Config) {
    println!("[Configured Profiles]");

    if config.default_profiles.is_empty() {
        println!("[INFO] No default report profiles configured.");
    } else {
        for profile in &config.default_profiles {
            println!("- {}", profile);
        }
    }

    println!();
}

fn inspect_udp(config: &Config, issues: &mut Vec<String>) {
    println!("[UDP Diagnostics]");

    let udp = run_udp_diagnostics(&config.udp_test_target);

    if let Some(err) = udp.bind_error {
        println!("[FAILED] UDP socket bind: {}", err);
        issues.push("UDP socket bind failed.".to_string());
    } else {
        println!("[OK] UDP socket bind: OK");
    }

    if let Some(err) = udp.connect_error {
        println!("[FAILED] UDP connect test: {}", err);
        issues.push("UDP connect test failed.".to_string());
    } else {
        println!("[OK] UDP connect test: OK");
    }

    println!();
}

fn inspect_network_info() {
    println!("[Network Info]");

    let adapters = get_network_adapters();

    if adapters.is_empty() {
        println!("[INFO] No active adapter details found.");
    } else {
        println!("[INFO] Active adapter entries found: {}", adapters.len());
        for adapter in adapters.iter().take(3) {
            println!(
                "- {} | IPv4: {} | Gateway: {}",
                known_or_unknown(&adapter.name),
                known_or_unknown(&adapter.ipv4_address),
                known_or_unknown(&adapter.default_gateway)
            );
        }
    }

    println!();
}

fn inspect_firewall_summary(config: &Config) {
    println!("[Firewall Summary]");

    let results = inspect_firewall_targets(config);
    let total_rules: usize = results.iter().map(|(_, rules)| rules.len()).sum();
    println!("[INFO] Matching firewall rule count: {}", total_rules);

    println!();
}

fn inspect_services_summary(
    config: &Config,
    issues: &mut Vec<String>,
    suggestions: &mut Vec<String>,
) {
    println!("[Services Summary]");

    for (target, service) in inspect_services(config) {
        match service {
            Some(service) => {
                println!(
                    "- {}: {} ({})",
                    service.name,
                    known_or_unknown(&service.status),
                    known_or_unknown(&service.start_type)
                );

                if service.name.eq_ignore_ascii_case("warp-svc")
                    && service.status.eq_ignore_ascii_case("running")
                    && config.show_warp_warning
                {
                    issues
                        .push("Cloudflare WARP service is running in the background.".to_string());
                    suggestions.push(
                        "If you are not using Cloudflare WARP, close or disable it from the official app."
                            .to_string(),
                    );
                }
            }
            None => println!("- {}: Not found", target),
        }
    }

    println!();
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();

    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }

    unique
}

fn known_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "Unknown"
    } else {
        value
    }
}
