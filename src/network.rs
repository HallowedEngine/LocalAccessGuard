use crate::config::Config;
use crate::profiles::load_profile;
use crate::windows::check_processes;
use serde::Serialize;
use std::fmt::Write as _;
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const UDP_NOTE: &str = "This is a basic local UDP capability check. It does not guarantee that every game or voice service UDP path is reachable.";

#[derive(Debug, Serialize)]
pub(crate) struct UdpDiagnosticResult {
    pub(crate) bind_error: Option<String>,
    pub(crate) local_socket: Option<String>,
    pub(crate) connect_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct NetworkAdapterInfo {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) dhcp_enabled: String,
    pub(crate) ipv4_address: String,
    pub(crate) default_gateway: String,
    pub(crate) dns_servers: Vec<String>,
}

pub fn netinfo() {
    println!("=== Network Info ===");
    println!();

    let adapters = get_network_adapters();
    print_network_adapters(&adapters);
}

pub fn udpcheck(config: &Config) {
    println!("=== UDP Diagnostics ===");
    println!();

    let diagnostics = run_udp_diagnostics(&config.udp_test_target);
    print_udp_diagnostics(&diagnostics, &config.udp_test_target);
}

pub(crate) fn run_udp_diagnostics(udp_test_target: &str) -> UdpDiagnosticResult {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(err) => {
            return UdpDiagnosticResult {
                bind_error: Some(err.to_string()),
                local_socket: None,
                connect_error: Some(format!("bind failed before connect: {}", err)),
            };
        }
    };

    let local_socket = socket.local_addr().ok().map(|addr| addr.to_string());
    let connect_error = socket
        .connect(udp_test_target)
        .err()
        .map(|err| err.to_string());

    UdpDiagnosticResult {
        bind_error: None,
        local_socket,
        connect_error,
    }
}

fn print_udp_diagnostics(diagnostics: &UdpDiagnosticResult, udp_test_target: &str) {
    match &diagnostics.bind_error {
        Some(err) => {
            println!("[FAILED] UDP socket bind: FAILED ({})", err);
        }
        None => {
            println!("[OK] UDP socket bind: OK");
        }
    }

    println!(
        "[INFO] Local UDP socket: {}",
        diagnostics.local_socket.as_deref().unwrap_or("Unavailable")
    );

    match &diagnostics.connect_error {
        Some(err) => {
            println!("[FAILED] UDP connect test: FAILED ({})", err);
        }
        None => {
            println!("[OK] UDP connect test: OK");
        }
    }

    println!("[INFO] UDP test target: {}", udp_test_target);
    println!("[INFO] Note: {}", UDP_NOTE);
}

pub(crate) fn write_udp_diagnostics(
    output: &mut String,
    diagnostics: &UdpDiagnosticResult,
    udp_test_target: &str,
) {
    match &diagnostics.bind_error {
        Some(err) => {
            let _ = writeln!(output, "UDP socket bind: FAILED ({})", err);
        }
        None => {
            let _ = writeln!(output, "UDP socket bind: OK");
        }
    }

    let _ = writeln!(
        output,
        "Local UDP socket: {}",
        diagnostics.local_socket.as_deref().unwrap_or("Unavailable")
    );

    match &diagnostics.connect_error {
        Some(err) => {
            let _ = writeln!(output, "UDP connect test: FAILED ({})", err);
        }
        None => {
            let _ = writeln!(output, "UDP connect test: OK");
        }
    }

    let _ = writeln!(output, "UDP test target: {}", udp_test_target);
    let _ = writeln!(output, "Note: {}", UDP_NOTE);
}

pub fn doctor_profile(profile_name: &str, config: &Config) {
    let profile = match load_profile(profile_name, &config.profile_directory) {
        Ok(profile) => profile,
        Err(err) => {
            println!(
                "[FAILED] Failed to load profile '{}': {}",
                profile_name, err
            );
            let expected_path =
                Path::new(&config.profile_directory).join(format!("{}.json", profile_name));
            println!("[INFO] Expected file: {}", expected_path.to_string_lossy());
            return;
        }
    };

    println!("=== Doctor: {} ===", profile.name);
    println!();

    for domain in &profile.domains {
        check_domain(domain);
    }

    check_tcp_443(&profile.tcp_test_domain);
    check_processes(config);
}

fn check_domain(domain: &str) {
    println!("[DNS Test: {}]", domain);

    let address = format!("{}:443", domain);

    match address.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                println!("  [OK] Resolved: {}", addr.ip());
            } else {
                println!("  [FAILED] Failed: no address returned");
            }
        }
        Err(err) => {
            println!("  [FAILED] Failed: {}", err);
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
                    println!("  [OK] TCP 443: OK ({})", addr);
                    success = true;
                    break;
                }
            }

            if !success {
                println!("  [FAILED] TCP 443: Failed");
            }
        }
        Err(err) => {
            println!("  [FAILED] Could not resolve address: {}", err);
        }
    }

    println!();
}

pub(crate) fn get_dns_result(domain: &str) -> String {
    let address = format!("{}:443", domain);

    match address.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => format!("OK ({})", addr.ip()),
            None => "FAILED - no address returned".to_string(),
        },
        Err(err) => format!("FAILED - {}", err),
    }
}

pub(crate) fn get_tcp_443_result(domain: &str) -> String {
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

pub(crate) fn is_ok_result(result: &str) -> bool {
    result.starts_with("OK")
}

pub(crate) fn get_network_adapters() -> Vec<NetworkAdapterInfo> {
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

pub(crate) fn known_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "Unknown"
    } else {
        value
    }
}
