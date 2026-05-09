use crate::config::Config;
use crate::logger;
use std::fs;

#[derive(Debug)]
struct ParsedReportSummary {
    profiles_tested: usize,
    dns_failures: usize,
    tcp_failures: usize,
    warnings: usize,
    overall_status: String,
    reasons: Vec<String>,
}

pub fn compare_reports(old_report_path: &str, new_report_path: &str, config: &Config) {
    let old_text = match fs::read_to_string(old_report_path) {
        Ok(text) => text,
        Err(err) => {
            logger::error(
                config,
                &format!("failed to read report file: {}", old_report_path),
            );
            println!(
                "[FAILED] Failed to read old report '{}': {}",
                old_report_path, err
            );
            return;
        }
    };

    let new_text = match fs::read_to_string(new_report_path) {
        Ok(text) => text,
        Err(err) => {
            logger::error(
                config,
                &format!("failed to read report file: {}", new_report_path),
            );
            println!(
                "[FAILED] Failed to read new report '{}': {}",
                new_report_path, err
            );
            return;
        }
    };

    let old_summary = match parse_report_summary(&old_text) {
        Ok(summary) => summary,
        Err(err) => {
            println!("[FAILED] Invalid old report '{}': {}", old_report_path, err);
            return;
        }
    };

    let new_summary = match parse_report_summary(&new_text) {
        Ok(summary) => summary,
        Err(err) => {
            println!("[FAILED] Invalid new report '{}': {}", new_report_path, err);
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
        println!("[OK] No summary changes detected.");
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
