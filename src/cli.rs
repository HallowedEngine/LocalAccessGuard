use crate::VERSION;
use crate::compare::compare_reports;
use crate::config::{Config, EffectiveConfig};
use crate::groups::{doctor_group, groups};
use crate::logger;
use crate::network::{doctor_profile, netinfo, udpcheck};
use crate::profiles::{profiles, validate_profiles};
use crate::report::{ReportFormat, report};
use crate::windows::{restore, status};

pub fn handle_args(args: &[String], effective_config: EffectiveConfig) {
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "help" | "--help" | "-h" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=help");
            print_help();
        }
        "status" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=status");
            status(&config);
        }
        "doctor" => {
            if args.len() < 3 {
                println!("[FAILED] Missing profile name.");
                println!("Usage: LocalAccessGuard doctor <profile>");
                println!("Example: LocalAccessGuard doctor discord");
                println!("Example: LocalAccessGuard doctor roblox");
                return;
            }

            let config = config_with_warning(effective_config);
            logger::info(&config, &format!("command=doctor profile={}", args[2]));
            doctor_profile(&args[2], &config);
        }
        "doctor-group" => {
            if args.len() < 3 {
                println!("[FAILED] Missing group name.");
                println!("Usage: LocalAccessGuard doctor-group <group>");
                println!("Example: LocalAccessGuard doctor-group gaming");
                return;
            }

            let config = config_with_warning(effective_config);
            logger::info(&config, &format!("command=doctor-group group={}", args[2]));
            doctor_group(&args[2], &config);
        }
        "profiles" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=profiles");
            profiles(&config);
        }
        "groups" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=groups");
            groups(&config);
        }
        "validate" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=validate");
            validate_profiles(&config);
        }
        "restore" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=restore");
            restore(&config);
        }
        "report" => {
            let config = config_with_warning(effective_config);
            let format = match parse_report_format(args) {
                Ok(format) => format,
                Err(format) => {
                    logger::info(&config, &format!("command=report format={}", format));
                    println!("[FAILED] Unsupported report format: {}", format);
                    println!("Supported formats: txt, json, md");
                    return;
                }
            };

            logger::info(
                &config,
                &format!("command=report format={}", format.as_str()),
            );
            report(&config, format);
        }
        "netinfo" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=netinfo");
            netinfo();
        }
        "udpcheck" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=udpcheck");
            udpcheck(&config);
        }
        "config" => {
            let config = config_with_warning(effective_config);
            logger::info(&config, "command=config");
            crate::config::print_config();
        }
        "compare" => {
            if args.len() < 4 {
                println!("[FAILED] Missing report path.");
                println!("Usage: LocalAccessGuard compare <old_report> <new_report>");
                println!("Example: LocalAccessGuard compare reports\\old.txt reports\\new.txt");
                return;
            }

            let config = config_with_warning(effective_config);
            logger::info(
                &config,
                &format!("command=compare old={} new={}", args[2], args[3]),
            );
            compare_reports(&args[2], &args[3], &config);
        }
        _ => print_help(),
    }
}

fn config_with_warning(effective_config: EffectiveConfig) -> Config {
    effective_config.into_config_with_warning()
}

fn print_help() {
    println!("LocalAccessGuard {}", VERSION);
    println!();
    println!("Usage:");
    println!("  LocalAccessGuard <command>");
    println!();
    println!("Commands:");
    println!("  status                          Show system proxy and process status");
    println!("  restore                         Restore proxy/PAC/WinHTTP settings");
    println!("  profiles                        List available JSON profiles");
    println!("  validate                        Validate profile JSON files");
    println!("  doctor <profile>                Run DNS/TCP diagnostics for a profile");
    println!("  doctor-group <group>            Run doctor checks for a profile group");
    println!("  report                          Generate timestamped diagnostic report");
    println!("  report --format <txt|json|md>     Generate report in selected format");
    println!("  groups                            List profile groups");
    println!("  compare <old_report> <new_report> Compare two diagnostic reports");
    println!("  netinfo                         Show active adapter and DNS information");
    println!("  udpcheck                        Run basic UDP diagnostics");
    println!("  config                          Show effective configuration");
    println!("  help                            Show this help screen");
    println!();
    println!("Examples:");
    println!("  LocalAccessGuard status");
    println!("  LocalAccessGuard doctor discord");
    println!("  LocalAccessGuard doctor-group gaming");
    println!("  LocalAccessGuard report");
    println!("  LocalAccessGuard report --format json");
    println!("  LocalAccessGuard compare reports\\old.txt reports\\new.txt");
}

fn parse_report_format(args: &[String]) -> Result<ReportFormat, String> {
    if args.len() == 2 {
        return Ok(ReportFormat::Txt);
    }

    if args.len() == 4 && args[2] == "--format" {
        return ReportFormat::parse(&args[3]).ok_or_else(|| args[3].clone());
    }

    let provided = args.get(2).cloned().unwrap_or_default();
    Err(provided)
}
