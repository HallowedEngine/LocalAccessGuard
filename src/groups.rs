use crate::config::Config;
use crate::network::doctor_profile;
use crate::profiles::load_profile;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub(crate) struct Group {
    pub(crate) name: String,
    pub(crate) profiles: Vec<String>,
}

pub fn groups(config: &Config) {
    let entries = match group_files(&config.group_directory) {
        Ok(entries) => entries,
        Err(_) => {
            println!("[WARNING] No valid groups found.");
            return;
        }
    };

    let mut valid_groups = Vec::new();

    for path in entries {
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let group = match parse_and_validate_group(&text) {
            Ok(group) => group,
            Err(_) => continue,
        };

        valid_groups.push(group);
    }

    if valid_groups.is_empty() {
        println!("[WARNING] No valid groups found.");
        return;
    }

    valid_groups.sort_by(|left, right| left.name.cmp(&right.name));

    println!("[INFO] Available groups:");
    for group in valid_groups {
        println!("- {}: {}", group.name, group.profiles.join(", "));
    }
}

pub fn doctor_group(group_name: &str, config: &Config) {
    let group = match load_group(group_name, &config.group_directory) {
        Ok(group) => group,
        Err(_) => {
            println!("[FAILED] Group not found: {}", group_name);
            return;
        }
    };

    println!("=== Doctor Group: {} ===", group.name);
    println!();

    for profile_name in group.profiles {
        println!("[Group Profile: {}]", profile_name);

        if load_profile(&profile_name, &config.profile_directory).is_err() {
            println!("[FAILED] Profile in group not found: {}", profile_name);
            println!();
            continue;
        }

        doctor_profile(&profile_name, config);
    }
}

fn load_group(group_name: &str, group_directory: &str) -> Result<Group, String> {
    let file_path = Path::new(group_directory).join(format!("{}.json", group_name));
    let text = fs::read_to_string(&file_path)
        .map_err(|err| format!("could not read {}: {}", file_path.to_string_lossy(), err))?;

    parse_and_validate_group(&text)
}

fn group_files(group_directory: &str) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(group_directory).map_err(|err| err.to_string())?;
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

fn parse_and_validate_group(text: &str) -> Result<Group, String> {
    let group: Group =
        serde_json::from_str(text).map_err(|err| format!("invalid JSON: {}", err))?;

    if group.name.trim().is_empty() {
        return Err("missing or empty name".to_string());
    }

    if group.profiles.is_empty() {
        return Err("missing or empty profiles".to_string());
    }

    if group
        .profiles
        .iter()
        .any(|profile| profile.trim().is_empty())
    {
        return Err("profiles contains empty entry".to_string());
    }

    Ok(group)
}
