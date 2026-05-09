use crate::config::Config;
use crate::logger;
use std::collections::BTreeMap;
use std::process::Command;

const SERVICE_TARGETS: [&str; 5] = [
    "warp-svc",
    "Cloudflare WARP",
    "WinHttpAutoProxySvc",
    "Dhcp",
    "Dnscache",
];

#[derive(Debug, Clone)]
pub(crate) struct ServiceInfo {
    pub(crate) name: String,
    pub(crate) display_name: String,
    pub(crate) status: String,
    pub(crate) start_type: String,
    pub(crate) binary_path: String,
}

pub fn services(config: &Config) {
    println!("=== Services Inspection ===");
    println!();

    for target in SERVICE_TARGETS {
        println!("[Service: {}]", target);

        match inspect_service_target(target) {
            Ok(Some(service)) => print_service(&service),
            Ok(None) => println!("[INFO] Service not found: {}", target),
            Err(err) => {
                println!(
                    "[WARNING] Service inspection command failed for {}: {}",
                    target, err
                );
                logger::warning(
                    config,
                    &format!(
                        "service inspection command failure target={} error={}",
                        target, err
                    ),
                );
            }
        }

        println!();
    }
}

pub(crate) fn inspect_services(config: &Config) -> Vec<(String, Option<ServiceInfo>)> {
    let mut results = Vec::new();

    for target in SERVICE_TARGETS {
        match inspect_service_target(target) {
            Ok(service) => results.push((target.to_string(), service)),
            Err(err) => {
                logger::warning(
                    config,
                    &format!(
                        "service inspection command failure target={} error={}",
                        target, err
                    ),
                );
                results.push((target.to_string(), None));
            }
        }
    }

    dedupe_services(results)
}

fn inspect_service_target(target: &str) -> Result<Option<ServiceInfo>, String> {
    let command = format!(
        "$target = '{}'; \
         $service = Get-CimInstance Win32_Service -ErrorAction Stop | Where-Object {{ $_.Name -ieq $target -or $_.DisplayName -ieq $target }} | Select-Object -First 1; \
         if ($service) {{ @($service.Name,$service.DisplayName,$service.State,$service.StartMode,$service.PathName) -join \"`t\" }}",
        escape_powershell_single_quoted(target)
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &command])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(err) => return Err(err.to_string()),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        let detail = if detail.is_empty() {
            format!("PowerShell exited with status {}", output.status)
        } else {
            detail
        };
        return Err(detail);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_service_info(&text))
}

fn print_service(service: &ServiceInfo) {
    println!("Service Name: {}", known_or_unknown(&service.name));
    println!("Display Name: {}", known_or_unknown(&service.display_name));
    println!("Status: {}", known_or_unknown(&service.status));
    println!("Start Type: {}", known_or_unknown(&service.start_type));

    if !service.binary_path.trim().is_empty() {
        println!("Binary Path: {}", service.binary_path);
    }
}

fn parse_service_info(text: &str) -> Option<ServiceInfo> {
    let line = text.lines().find(|line| !line.trim().is_empty())?;
    let parts: Vec<&str> = line.trim().split('\t').collect();

    Some(ServiceInfo {
        name: parts.first().unwrap_or(&"").trim().to_string(),
        display_name: parts.get(1).unwrap_or(&"").trim().to_string(),
        status: parts.get(2).unwrap_or(&"").trim().to_string(),
        start_type: parts.get(3).unwrap_or(&"").trim().to_string(),
        binary_path: parts.get(4).unwrap_or(&"").trim().to_string(),
    })
}

fn dedupe_services(
    results: Vec<(String, Option<ServiceInfo>)>,
) -> Vec<(String, Option<ServiceInfo>)> {
    let mut seen = BTreeMap::new();
    let mut deduped = Vec::new();

    for (target, service) in results {
        if let Some(service) = &service {
            let key = service.name.to_lowercase();

            if seen.insert(key, true).is_some() {
                continue;
            }
        }

        deduped.push((target, service));
    }

    deduped
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn known_or_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "Unknown"
    } else {
        value
    }
}
