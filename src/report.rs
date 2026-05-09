use crate::VERSION;
use crate::config::Config;
use crate::logger;
use crate::network::{
    NetworkAdapterInfo, UdpDiagnosticResult, get_dns_result, get_network_adapters,
    get_tcp_443_result, is_ok_result, known_or_unknown, run_udp_diagnostics, write_udp_diagnostics,
};
use crate::profiles::load_report_profiles;
use crate::windows::read_reg_value;
use chrono::Local;
use serde::Serialize;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ReportFormat {
    Txt,
    Json,
    Md,
}

impl ReportFormat {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "txt" => Some(Self::Txt),
            "json" => Some(Self::Json),
            "md" | "markdown" => Some(Self::Md),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Json => "json",
            Self::Md => "md",
        }
    }

    fn extension(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
struct JsonReport {
    version: String,
    generated: String,
    summary: ReportSummary,
    windows_proxy: String,
    auto_proxy: String,
    winhttp_proxy: String,
    network_info: Vec<NetworkAdapterInfo>,
    udp_diagnostics: UdpDiagnosticResult,
    known_network_tools: Vec<ProcessReport>,
    profiles: Vec<ProfileReport>,
}

#[derive(Debug, Serialize)]
struct ProcessReport {
    name: String,
    running: bool,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ProfileReport {
    name: String,
    domains: Vec<DomainReport>,
    tcp_test_domain: String,
    tcp_443: String,
}

#[derive(Debug, Serialize)]
struct DomainReport {
    domain: String,
    dns: String,
}

pub fn report(config: &Config, format: ReportFormat) {
    println!("=== LocalAccessGuard Report ===");
    println!();

    let report_content = match format {
        ReportFormat::Txt => build_report_text(config),
        ReportFormat::Json => match build_report_json(config) {
            Ok(json) => json,
            Err(err) => {
                println!("[FAILED] Failed to build JSON report: {}", err);
                return;
            }
        },
        ReportFormat::Md => build_report_markdown(config),
    };

    match fs::create_dir_all(&config.report_directory) {
        Ok(_) => {}
        Err(err) => {
            println!("[FAILED] Failed to create reports directory: {}", err);
            return;
        }
    }

    let file_path = unique_report_path(&config.report_directory, format);

    let file_result = File::create(&file_path);

    let mut file = match file_result {
        Ok(file) => file,
        Err(err) => {
            println!("[FAILED] Failed to create report file: {}", err);
            return;
        }
    };

    match file.write_all(report_content.as_bytes()) {
        Ok(_) => {
            println!("[OK] Report saved:");
            println!("{}", file_path.to_string_lossy());
            logger::info(
                config,
                &format!(
                    "command=report format={} path={}",
                    format.as_str(),
                    file_path.to_string_lossy()
                ),
            );
        }
        Err(err) => {
            println!("[FAILED] Failed to write report file: {}", err);
        }
    }
}

fn unique_report_path(report_directory: &str, format: ReportFormat) -> PathBuf {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S_%3f");
    let base_name = format!("local_access_report_{}", timestamp);
    let directory = Path::new(report_directory);
    let mut path = directory.join(format!("{}.{}", base_name, format.extension()));

    if !path.exists() {
        return path;
    }

    let mut suffix = 1;
    loop {
        path = directory.join(format!("{}_{}.{}", base_name, suffix, format.extension()));

        if !path.exists() {
            return path;
        }

        suffix += 1;
    }
}

fn build_report_text(config: &Config) -> String {
    let mut report = String::new();
    let summary = build_report_summary(config);

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
    append_udp_diagnostics_report(&mut report, config);
    append_process_report(&mut report, config);
    append_profiles_report(&mut report, config);

    report
}

fn build_report_markdown(config: &Config) -> String {
    let mut report = String::new();
    let summary = build_report_summary(config);

    let _ = writeln!(report, "# LocalAccessGuard Report");
    let _ = writeln!(report);
    let _ = writeln!(report, "- Version: {}", VERSION);
    let _ = writeln!(
        report,
        "- Generated: {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let _ = writeln!(report);
    append_markdown_summary(&mut report, &summary);

    append_markdown_section(&mut report, "Windows Proxy", &section_windows_proxy());
    append_markdown_section(&mut report, "Auto Proxy / PAC", &section_autoconfig());
    append_markdown_section(&mut report, "WinHTTP Proxy", &section_winhttp());
    append_markdown_network_info(&mut report);
    append_markdown_udp_diagnostics(&mut report, config);
    append_markdown_processes(&mut report, config);
    append_markdown_profiles(&mut report, config);

    report
}

fn build_report_json(config: &Config) -> Result<String, serde_json::Error> {
    let report = JsonReport {
        version: VERSION.to_string(),
        generated: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        summary: build_report_summary(config),
        windows_proxy: section_windows_proxy(),
        auto_proxy: section_autoconfig(),
        winhttp_proxy: section_winhttp(),
        network_info: get_network_adapters(),
        udp_diagnostics: run_udp_diagnostics(&config.udp_test_target),
        known_network_tools: collect_process_report(config),
        profiles: collect_profile_report(config),
    };

    serde_json::to_string_pretty(&report)
}

fn build_report_summary(config: &Config) -> ReportSummary {
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

    add_process_warnings(&mut summary, config);
    add_profile_results(&mut summary, config);

    summary
}

fn add_process_warnings(summary: &mut ReportSummary, config: &Config) {
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

        if process == "warp-svc.exe" && config.show_warp_warning {
            summary.add_warning("Cloudflare WARP service is running in the background.");
            logger::warning(
                config,
                "Cloudflare WARP service is running in the background.",
            );
        } else if process != "warp-svc.exe" {
            summary.add_warning(&format!("DPI/proxy tool process is active: {}", process));
        }
    }
}

fn append_markdown_summary(report: &mut String, summary: &ReportSummary) {
    let _ = writeln!(report, "## Summary");
    let _ = writeln!(report);
    let _ = writeln!(report, "- Profiles tested: {}", summary.profiles_tested);
    let _ = writeln!(report, "- DNS failures: {}", summary.dns_failures);
    let _ = writeln!(report, "- TCP failures: {}", summary.tcp_failures);
    let _ = writeln!(report, "- Warnings: {}", summary.warnings);
    let _ = writeln!(report, "- Overall status: {}", summary.overall_status());
    let _ = writeln!(report, "- Reasons:");

    if summary.reasons.is_empty() {
        let _ = writeln!(report, "  - None.");
    } else {
        for reason in &summary.reasons {
            let _ = writeln!(report, "  - {}", reason);
        }
    }

    let _ = writeln!(report);
}

fn append_markdown_section(report: &mut String, heading: &str, text: &str) {
    let _ = writeln!(report, "## {}", heading);
    let _ = writeln!(report);

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let _ = writeln!(report, "- {}", line);
    }

    let _ = writeln!(report);
}

fn append_markdown_network_info(report: &mut String) {
    let _ = writeln!(report, "## Network Info");
    let _ = writeln!(report);
    let adapters = get_network_adapters();

    if adapters.is_empty() {
        let _ = writeln!(report, "- Adapter Name: Unknown");
        let _ = writeln!(report, "- Description: Unknown");
        let _ = writeln!(report, "- DHCP Enabled: Unknown");
        let _ = writeln!(report, "- IPv4 Address: Unknown");
        let _ = writeln!(report, "- Default Gateway: Unknown");
        let _ = writeln!(report, "- DNS Servers: Unknown");
        let _ = writeln!(report);
        return;
    }

    for adapter in adapters {
        let dns_servers = if adapter.dns_servers.is_empty() {
            "Unknown".to_string()
        } else {
            adapter.dns_servers.join(", ")
        };

        let _ = writeln!(
            report,
            "- Adapter Name: {}",
            known_or_unknown(&adapter.name)
        );
        let _ = writeln!(
            report,
            "  - Description: {}",
            known_or_unknown(&adapter.description)
        );
        let _ = writeln!(
            report,
            "  - DHCP Enabled: {}",
            known_or_unknown(&adapter.dhcp_enabled)
        );
        let _ = writeln!(
            report,
            "  - IPv4 Address: {}",
            known_or_unknown(&adapter.ipv4_address)
        );
        let _ = writeln!(
            report,
            "  - Default Gateway: {}",
            known_or_unknown(&adapter.default_gateway)
        );
        let _ = writeln!(report, "  - DNS Servers: {}", dns_servers);
    }

    let _ = writeln!(report);
}

fn append_markdown_udp_diagnostics(report: &mut String, config: &Config) {
    let diagnostics = run_udp_diagnostics(&config.udp_test_target);
    let mut text = String::new();
    write_udp_diagnostics(&mut text, &diagnostics, &config.udp_test_target);
    append_markdown_section(report, "UDP Diagnostics", &text);
}

fn append_markdown_processes(report: &mut String, config: &Config) {
    let _ = writeln!(report, "## Known Network Tools");
    let _ = writeln!(report);

    for process in collect_process_report(config) {
        let status = if process.running {
            "Running"
        } else {
            "Not running"
        };
        let _ = writeln!(report, "- {}: {}", process.name, status);

        for warning in process.warnings {
            let _ = writeln!(report, "  - Warning: {}", warning);
        }
    }

    let _ = writeln!(report);
}

fn append_markdown_profiles(report: &mut String, config: &Config) {
    let profiles = collect_profile_report(config);

    if profiles.is_empty() {
        let _ = writeln!(report, "## Profiles");
        let _ = writeln!(report);
        let _ = writeln!(
            report,
            "- No valid profiles found in {}\\*.json",
            config.profile_directory
        );
        let _ = writeln!(report);
        return;
    }

    for profile in profiles {
        let _ = writeln!(report, "## {}", profile.name);
        let _ = writeln!(report);

        for domain in profile.domains {
            let _ = writeln!(report, "- DNS {}: {}", domain.domain, domain.dns);
        }

        let _ = writeln!(
            report,
            "- TCP 443 {}: {}",
            profile.tcp_test_domain, profile.tcp_443
        );
        let _ = writeln!(report);
    }
}

fn section_windows_proxy() -> String {
    let mut report = String::new();
    append_windows_proxy_report(&mut report);
    strip_section_heading(&report)
}

fn section_autoconfig() -> String {
    let mut report = String::new();
    append_autoconfig_report(&mut report);
    strip_section_heading(&report)
}

fn section_winhttp() -> String {
    let mut report = String::new();
    append_winhttp_report(&mut report);
    strip_section_heading(&report)
}

fn strip_section_heading(text: &str) -> String {
    text.lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn collect_process_report(config: &Config) -> Vec<ProcessReport> {
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
    let text = match output {
        Ok(result) => String::from_utf8_lossy(&result.stdout).to_lowercase(),
        Err(_) => String::new(),
    };

    let mut processes = Vec::new();

    for process in known_processes {
        let running = text.contains(&process.to_lowercase());
        let mut warnings = Vec::new();

        if running && process == "warp-svc.exe" && config.show_warp_warning {
            let warning = "Cloudflare WARP service is running in the background.".to_string();
            logger::warning(config, &warning);
            warnings.push(warning);
        }

        if running
            && (process == "goodbyedpi.exe"
                || process == "bypax-proxy.exe"
                || process == "BypaxDPI.exe")
        {
            warnings.push(format!("DPI/proxy tool process is active: {}", process));
        }

        processes.push(ProcessReport {
            name: process.to_string(),
            running,
            warnings,
        });
    }

    processes
}

fn collect_profile_report(config: &Config) -> Vec<ProfileReport> {
    let profiles = load_report_profiles(config);
    let mut report_profiles = Vec::new();

    for profile in profiles {
        let mut domains = Vec::new();

        for domain in &profile.domains {
            domains.push(DomainReport {
                domain: domain.clone(),
                dns: get_dns_result(domain),
            });
        }

        let tcp_443 = get_tcp_443_result(&profile.tcp_test_domain);

        report_profiles.push(ProfileReport {
            name: profile.name,
            domains,
            tcp_test_domain: profile.tcp_test_domain,
            tcp_443,
        });
    }

    report_profiles
}

fn add_profile_results(summary: &mut ReportSummary, config: &Config) {
    let profiles = load_report_profiles(config);
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

fn append_udp_diagnostics_report(report: &mut String, config: &Config) {
    let diagnostics = run_udp_diagnostics(&config.udp_test_target);

    let _ = writeln!(report, "[UDP Diagnostics]");
    write_udp_diagnostics(report, &diagnostics, &config.udp_test_target);
    let _ = writeln!(report);
}

fn append_process_report(report: &mut String, config: &Config) {
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

                    if process == "warp-svc.exe" && config.show_warp_warning {
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

fn append_profiles_report(report: &mut String, config: &Config) {
    let profiles = load_report_profiles(config);

    if profiles.is_empty() {
        let _ = writeln!(report, "[Profiles]");
        let _ = writeln!(
            report,
            "No valid profiles found in {}\\*.json",
            config.profile_directory
        );
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
