use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = "config.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub profile_directory: String,
    pub group_directory: String,
    pub report_directory: String,
    pub log_directory: String,
    pub default_profiles: Vec<String>,
    pub udp_test_target: String,
    pub show_warp_warning: bool,
    pub enable_logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile_directory: "profiles".to_string(),
            group_directory: "groups".to_string(),
            report_directory: "reports".to_string(),
            log_directory: "logs".to_string(),
            default_profiles: vec!["discord".to_string(), "roblox".to_string()],
            udp_test_target: "1.1.1.1:53".to_string(),
            show_warp_warning: true,
            enable_logging: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    profile_directory: Option<String>,
    group_directory: Option<String>,
    report_directory: Option<String>,
    log_directory: Option<String>,
    default_profiles: Option<Vec<String>>,
    udp_test_target: Option<String>,
    show_warp_warning: Option<bool>,
    enable_logging: Option<bool>,
}

impl From<ConfigFile> for Config {
    fn from(file: ConfigFile) -> Self {
        let defaults = Config::default();

        Self {
            profile_directory: file.profile_directory.unwrap_or(defaults.profile_directory),
            group_directory: file.group_directory.unwrap_or(defaults.group_directory),
            report_directory: file.report_directory.unwrap_or(defaults.report_directory),
            log_directory: file.log_directory.unwrap_or(defaults.log_directory),
            default_profiles: file.default_profiles.unwrap_or(defaults.default_profiles),
            udp_test_target: file.udp_test_target.unwrap_or(defaults.udp_test_target),
            show_warp_warning: file.show_warp_warning.unwrap_or(defaults.show_warp_warning),
            enable_logging: file.enable_logging.unwrap_or(defaults.enable_logging),
        }
    }
}

#[derive(Debug)]
enum ConfigSource {
    File,
    Missing,
    Invalid(String),
}

#[derive(Debug)]
pub struct EffectiveConfig {
    config: Config,
    source: ConfigSource,
}

impl EffectiveConfig {
    pub fn into_config_with_warning(self) -> Config {
        if let ConfigSource::Invalid(err) = &self.source {
            println!("[WARNING] Config source: built-in defaults; {}.", err);
            crate::logger::warning(
                &self.config,
                &format!("invalid config warning: built-in defaults; {}", err),
            );
        }

        self.config
    }
}

pub fn load_config() -> EffectiveConfig {
    let config_path = Path::new(CONFIG_PATH);

    if !config_path.exists() {
        return EffectiveConfig {
            config: Config::default(),
            source: ConfigSource::Missing,
        };
    }

    let text = match fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(err) => {
            return EffectiveConfig {
                config: Config::default(),
                source: ConfigSource::Invalid(format!("could not read {}: {}", CONFIG_PATH, err)),
            };
        }
    };

    match serde_json::from_str::<ConfigFile>(&text) {
        Ok(config_file) => EffectiveConfig {
            config: config_file.into(),
            source: ConfigSource::File,
        },
        Err(err) => EffectiveConfig {
            config: Config::default(),
            source: ConfigSource::Invalid(format!("invalid {}: {}", CONFIG_PATH, err)),
        },
    }
}

pub fn print_config() {
    let effective = load_config();

    println!("=== LocalAccessGuard Config ===");
    println!();

    match &effective.source {
        ConfigSource::File => println!("[INFO] Config source: {}", CONFIG_PATH),
        ConfigSource::Missing => println!(
            "[WARNING] Config source: built-in defaults; {} not found.",
            CONFIG_PATH
        ),
        ConfigSource::Invalid(err) => {
            println!("[WARNING] Config source: built-in defaults; {}.", err)
        }
    }

    println!();
    println!("profile_directory: {}", effective.config.profile_directory);
    println!("group_directory: {}", effective.config.group_directory);
    println!("report_directory: {}", effective.config.report_directory);
    println!("log_directory: {}", effective.config.log_directory);
    println!("default_profiles:");

    for profile in &effective.config.default_profiles {
        println!("- {}", profile);
    }

    println!("udp_test_target: {}", effective.config.udp_test_target);
    println!("show_warp_warning: {}", effective.config.show_warp_warning);
    println!("enable_logging: {}", effective.config.enable_logging);
}
