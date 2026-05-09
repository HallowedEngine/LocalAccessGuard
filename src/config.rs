use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_PATH: &str = "config.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub profile_directory: String,
    pub report_directory: String,
    pub default_profiles: Vec<String>,
    pub udp_test_target: String,
    pub show_warp_warning: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile_directory: "profiles".to_string(),
            report_directory: "reports".to_string(),
            default_profiles: vec!["discord".to_string(), "roblox".to_string()],
            udp_test_target: "1.1.1.1:53".to_string(),
            show_warp_warning: true,
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

    match serde_json::from_str::<Config>(&text) {
        Ok(config) => EffectiveConfig {
            config,
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
    println!("report_directory: {}", effective.config.report_directory);
    println!("default_profiles:");

    for profile in &effective.config.default_profiles {
        println!("- {}", profile);
    }

    println!("udp_test_target: {}", effective.config.udp_test_target);
    println!("show_warp_warning: {}", effective.config.show_warp_warning);
}
