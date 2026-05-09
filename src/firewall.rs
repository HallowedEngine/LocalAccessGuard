use crate::config::Config;
use crate::logger;
use std::collections::BTreeMap;
use std::process::Command;

const FIREWALL_TARGETS: [&str; 6] = [
    "Discord.exe",
    "RobloxPlayerBeta.exe",
    "Cloudflare WARP.exe",
    "goodbyedpi.exe",
    "bypax-proxy.exe",
    "BypaxDPI.exe",
];

#[derive(Debug, Clone)]
pub(crate) struct FirewallRuleInfo {
    pub(crate) name: String,
    pub(crate) enabled: String,
    pub(crate) direction: String,
    pub(crate) action: String,
    pub(crate) profile: String,
    pub(crate) program: String,
}

pub fn firewall_check(config: &Config) {
    println!("=== Firewall Check ===");
    println!();

    for target in FIREWALL_TARGETS {
        println!("[Target: {}]", target);

        match inspect_firewall_target(target) {
            Ok(rules) => print_firewall_target(target, &rules),
            Err(err) => {
                println!(
                    "[WARNING] Firewall inspection command failed for {}: {}",
                    target, err
                );
                logger::warning(
                    config,
                    &format!(
                        "firewall inspection command failure target={} error={}",
                        target, err
                    ),
                );
            }
        }

        println!();
    }
}

pub(crate) fn inspect_firewall_targets(config: &Config) -> Vec<(String, Vec<FirewallRuleInfo>)> {
    let mut results = Vec::new();

    for target in FIREWALL_TARGETS {
        match inspect_firewall_target(target) {
            Ok(rules) => results.push((target.to_string(), rules)),
            Err(err) => {
                logger::warning(
                    config,
                    &format!(
                        "firewall inspection command failure target={} error={}",
                        target, err
                    ),
                );
                results.push((target.to_string(), Vec::new()));
            }
        }
    }

    results
}

fn inspect_firewall_target(target: &str) -> Result<Vec<FirewallRuleInfo>, String> {
    let command = format!(
        "$target = '{}'; \
         $stem = [System.IO.Path]::GetFileNameWithoutExtension($target); \
         $rules = @(); \
         $appFilters = Get-NetFirewallApplicationFilter -ErrorAction Stop | Where-Object {{ $_.Program -and ((Split-Path $_.Program -Leaf) -ieq $target -or $_.Program -like \"*$target*\") }}; \
         foreach ($filter in $appFilters) {{ \
           $rule = Get-NetFirewallRule -Name $filter.InstanceID -ErrorAction SilentlyContinue; \
           if ($rule) {{ $rules += [PSCustomObject]@{{ Name=$rule.DisplayName; Enabled=$rule.Enabled; Direction=$rule.Direction; Action=$rule.Action; Profile=$rule.Profile; Program=$filter.Program }} }} \
         }}; \
         $nameRules = Get-NetFirewallRule -ErrorAction Stop | Where-Object {{ $_.DisplayName -like \"*$stem*\" }}; \
         foreach ($rule in $nameRules) {{ \
           $filter = Get-NetFirewallApplicationFilter -AssociatedNetFirewallRule $rule -ErrorAction SilentlyContinue | Select-Object -First 1; \
           $program = if ($filter) {{ $filter.Program }} else {{ '' }}; \
           $rules += [PSCustomObject]@{{ Name=$rule.DisplayName; Enabled=$rule.Enabled; Direction=$rule.Direction; Action=$rule.Action; Profile=$rule.Profile; Program=$program }}; \
         }}; \
         $rules | Sort-Object Name,Direction,Action,Program -Unique | ForEach-Object {{ \
           @($_.Name,$_.Enabled,$_.Direction,$_.Action,$_.Profile,$_.Program) -join \"`t\" \
         }}",
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
    Ok(parse_firewall_rules(&text))
}

fn print_firewall_target(target: &str, rules: &[FirewallRuleInfo]) {
    if rules.is_empty() {
        println!("[INFO] No matching firewall rules found for {}", target);
        return;
    }

    println!("[INFO] Matching rules found: {}", rules.len());

    for rule in rules {
        println!("Rule: {}", known_or_unknown(&rule.name));
        println!("Enabled: {}", known_or_unknown(&rule.enabled));
        println!("Direction: {}", known_or_unknown(&rule.direction));
        println!("Action: {}", known_or_unknown(&rule.action));
        println!("Profile: {}", known_or_unknown(&rule.profile));

        if !rule.program.trim().is_empty() {
            println!("Program: {}", rule.program);
        }
    }
}

fn parse_firewall_rules(text: &str) -> Vec<FirewallRuleInfo> {
    let mut by_key: BTreeMap<String, FirewallRuleInfo> = BTreeMap::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split('\t').collect();

        if parts.len() < 5 {
            continue;
        }

        let rule = FirewallRuleInfo {
            name: parts.first().unwrap_or(&"").trim().to_string(),
            enabled: parts.get(1).unwrap_or(&"").trim().to_string(),
            direction: parts.get(2).unwrap_or(&"").trim().to_string(),
            action: parts.get(3).unwrap_or(&"").trim().to_string(),
            profile: parts.get(4).unwrap_or(&"").trim().to_string(),
            program: parts.get(5).unwrap_or(&"").trim().to_string(),
        };

        let key = format!(
            "{}|{}|{}|{}|{}",
            rule.name, rule.enabled, rule.direction, rule.action, rule.program
        );
        by_key.insert(key, rule);
    }

    by_key.into_values().collect()
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
