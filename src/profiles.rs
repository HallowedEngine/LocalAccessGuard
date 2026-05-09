use crate::config::Config;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub(crate) struct Profile {
    pub(crate) name: String,
    pub(crate) domains: Vec<String>,
    pub(crate) tcp_test_domain: String,
}

pub fn profiles(config: &Config) {
    let entries = match profile_files(&config.profile_directory) {
        Ok(entries) => entries,
        Err(_) => {
            println!("[WARNING] No valid profiles found.");
            return;
        }
    };

    let mut valid_profiles = Vec::new();

    for path in entries {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let profile = match parse_and_validate_profile(&text) {
            Ok(profile) => profile,
            Err(_) => continue,
        };

        let key = match profile_key_from_path(&path) {
            Some(key) => key,
            None => continue,
        };

        valid_profiles.push((key, profile.name));
    }

    if valid_profiles.is_empty() {
        println!("[WARNING] No valid profiles found.");
        return;
    }

    valid_profiles.sort_by(|left, right| left.0.cmp(&right.0));

    println!("[INFO] Available profiles:");
    for (key, name) in valid_profiles {
        println!("- {}: {}", key, name);
    }
}

pub fn validate_profiles(config: &Config) {
    let entries = match profile_files(&config.profile_directory) {
        Ok(entries) => entries,
        Err(err) => {
            println!("[FAILED] Could not read profiles directory: {}", err);
            return;
        }
    };

    println!("Profile validation:");

    for path in entries {
        let display_path = path.to_string_lossy();

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                println!(
                    "[FAILED] {}: INVALID - could not read file: {}",
                    display_path, err
                );
                continue;
            }
        };

        match parse_and_validate_profile(&text) {
            Ok(_) => println!("[OK] {}: OK", display_path),
            Err(err) => println!("[FAILED] {}: INVALID - {}", display_path, err),
        }
    }
}

pub(crate) fn load_profile(profile_name: &str, profile_directory: &str) -> Result<Profile, String> {
    let file_path = Path::new(profile_directory).join(format!("{}.json", profile_name));

    let text = fs::read_to_string(&file_path)
        .map_err(|err| format!("could not read {}: {}", file_path.to_string_lossy(), err))?;

    parse_and_validate_profile(&text)
}

fn load_all_profiles(profile_directory: &str) -> Vec<Profile> {
    let mut profiles = Vec::new();

    let entries = match profile_files(profile_directory) {
        Ok(entries) => entries,
        Err(_) => return profiles,
    };

    for path in entries {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let profile = match parse_and_validate_profile(&text) {
            Ok(profile) => profile,
            Err(_) => continue,
        };

        profiles.push(profile);
    }

    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    profiles
}

pub(crate) fn load_report_profiles(config: &Config) -> Vec<Profile> {
    let profiles = if config.default_profiles.is_empty() {
        load_all_profiles(&config.profile_directory)
    } else {
        let mut profiles = Vec::new();

        for profile_name in &config.default_profiles {
            if let Ok(profile) = load_profile(profile_name, &config.profile_directory) {
                profiles.push(profile);
            }
        }

        profiles
    };

    let mut profiles = profiles;
    profiles.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    profiles
}

fn profile_files(profile_directory: &str) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(profile_directory).map_err(|err| err.to_string())?;
    let mut paths = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();

        let is_json = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("json"))
            .unwrap_or(false);

        if is_json {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn profile_key_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
}

fn parse_and_validate_profile(text: &str) -> Result<Profile, String> {
    let profile: Profile =
        serde_json::from_str(text).map_err(|err| format!("invalid JSON: {}", err))?;

    validate_profile(&profile)?;

    Ok(profile)
}

fn validate_profile(profile: &Profile) -> Result<(), String> {
    if profile.name.trim().is_empty() {
        return Err("missing or empty name".to_string());
    }

    if profile.domains.is_empty() {
        return Err("missing or empty domains".to_string());
    }

    if profile
        .domains
        .iter()
        .any(|domain| domain.trim().is_empty())
    {
        return Err("domains contains empty entry".to_string());
    }

    if profile.tcp_test_domain.trim().is_empty() {
        return Err("missing or empty tcp_test_domain".to_string());
    }

    Ok(())
}
