use crate::VERSION;
use crate::compare::compare_reports;
use crate::config::{Config, EffectiveConfig};
use crate::network::{doctor_profile, netinfo, udpcheck};
use crate::profiles::{profiles, validate_profiles};
use crate::report::report;
use crate::windows::{restore, status};

pub fn handle_args(args: &[String], effective_config: EffectiveConfig) {
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "help" | "--help" | "-h" => print_help(),
        "status" => {
            let config = config_with_warning(effective_config);
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
            doctor_profile(&args[2], &config);
        }
        "profiles" => {
            let config = config_with_warning(effective_config);
            profiles(&config);
        }
        "validate" => {
            let config = config_with_warning(effective_config);
            validate_profiles(&config);
        }
        "restore" => restore(),
        "report" => {
            let config = config_with_warning(effective_config);
            report(&config);
        }
        "netinfo" => netinfo(),
        "udpcheck" => {
            let config = config_with_warning(effective_config);
            udpcheck(&config);
        }
        "config" => crate::config::print_config(),
        "compare" => {
            if args.len() < 4 {
                println!("[FAILED] Missing report path.");
                println!("Usage: LocalAccessGuard compare <old_report> <new_report>");
                println!("Example: LocalAccessGuard compare reports\\old.txt reports\\new.txt");
                return;
            }

            compare_reports(&args[2], &args[3]);
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
    println!("  report                          Generate timestamped diagnostic report");
    println!("  compare <old_report> <new_report> Compare two diagnostic reports");
    println!("  netinfo                         Show active adapter and DNS information");
    println!("  udpcheck                        Run basic UDP diagnostics");
    println!("  config                          Show effective configuration");
    println!("  help                            Show this help screen");
    println!();
    println!("Examples:");
    println!("  LocalAccessGuard status");
    println!("  LocalAccessGuard doctor discord");
    println!("  LocalAccessGuard report");
    println!("  LocalAccessGuard compare reports\\old.txt reports\\new.txt");
}
