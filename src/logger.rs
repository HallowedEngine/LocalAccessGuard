use crate::config::Config;
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const LOG_FILE_NAME: &str = "local_access_guard.log";

pub(crate) fn info(config: &Config, message: &str) {
    write_log(config, "INFO", message);
}

pub(crate) fn warning(config: &Config, message: &str) {
    write_log(config, "WARNING", message);
}

pub(crate) fn error(config: &Config, message: &str) {
    write_log(config, "ERROR", message);
}

fn write_log(config: &Config, level: &str, message: &str) {
    if !config.enable_logging {
        return;
    }

    if let Err(err) = fs::create_dir_all(&config.log_directory) {
        println!(
            "[WARNING] Failed to write log entry: could not create log directory: {}",
            err
        );
        return;
    }

    let log_path = Path::new(&config.log_directory).join(LOG_FILE_NAME);
    let mut file = match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(file) => file,
        Err(err) => {
            println!("[WARNING] Failed to write log entry: {}", err);
            return;
        }
    };

    let line = format!(
        "{} {} {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        level,
        message
    );

    if let Err(err) = file.write_all(line.as_bytes()) {
        println!("[WARNING] Failed to write log entry: {}", err);
    }
}
