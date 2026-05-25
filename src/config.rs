// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Global settings.
    pub global: GlobalConfig,
    /// Per-port configurations keyed by port path.
    #[serde(default)]
    pub ports: HashMap<String, PortConfig>,
    /// User-defined macros keyed by name.
    #[serde(default)]
    pub macros: HashMap<String, MacroConfig>,
}

/// Global server settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// Directory for database files.
    pub data_dir: PathBuf,
    /// Directory for archived databases.
    pub archive_dir: PathBuf,
    /// Flush interval in milliseconds.
    pub flush_interval_ms: u64,
    /// Maximum batch size before forced flush.
    pub flush_batch_size: usize,
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,
}

/// Per-port configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PortConfig {
    /// Baud rate.
    pub baudrate: u32,
    /// Data bits (5, 6, 7, 8).
    pub data_bits: u8,
    /// Parity: none, odd, even.
    pub parity: String,
    /// Stop bits (1, 2).
    pub stop_bits: u8,
    /// Flow control: none, software, hardware.
    pub flow_control: String,
    /// Line delimiter byte.
    pub delimiter: u8,
    /// Enable auto-reconnect on disconnect.
    pub auto_reconnect: bool,
    /// Reconnect interval in milliseconds.
    pub reconnect_interval_ms: u64,
    /// Maximum reconnect backoff in milliseconds.
    pub reconnect_max_backoff_ms: u64,
    /// Maximum lines to keep in buffer (0 = unlimited).
    pub max_buffer_lines: u64,
}

/// A user-defined macro: a sequence of steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroConfig {
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Ordered steps to execute.
    pub steps: Vec<MacroStep>,
}

/// A single step in a macro sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum MacroStep {
    /// Write bytes to the port (hex string or UTF-8).
    Write { value: String },
    /// Set DTR line.
    Dtr { value: bool },
    /// Set RTS line.
    Rts { value: bool },
    /// Delay in milliseconds.
    Delay { ms: u64 },
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            archive_dir: default_data_dir().join("archive"),
            flush_interval_ms: 100,
            flush_batch_size: 1000,
            log_level: "info".into(),
        }
    }
}

/// Platform-specific default data directory.
#[must_use]
pub fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map_or_else(
            || PathBuf::from("./data"),
            |h| PathBuf::from(h).join("Library/Application Support/devserial"),
        )
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_STATE_HOME")
            .map(|h| PathBuf::from(h).join("devserial"))
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state/devserial"))
            })
            .unwrap_or_else(|| PathBuf::from("./data"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map_or_else(
            || PathBuf::from("./data"),
            |h| PathBuf::from(h).join("devserial"),
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        PathBuf::from("./data")
    }
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            baudrate: 115_200,
            data_bits: 8,
            parity: "none".into(),
            stop_bits: 1,
            flow_control: "none".into(),
            delimiter: b'\n',
            auto_reconnect: true,
            reconnect_interval_ms: 1000,
            reconnect_max_backoff_ms: 30_000,
            max_buffer_lines: 0,
        }
    }
}

/// Load configuration from file discovery or CLI path.
///
/// Discovery order:
/// 1. Explicit path (if provided)
/// 2. `./devserial.toml`
/// 3. `~/.config/devserial/config.toml`
///
/// If no file is found, returns default config.
///
/// # Errors
/// Returns an error if a file exists but cannot be parsed.
pub fn load_config(explicit_path: Option<&Path>) -> Result<Config, ConfigError> {
    let candidates: Vec<PathBuf> = explicit_path.map_or_else(
        || {
            let mut c = vec![PathBuf::from("./devserial.toml")];
            if let Some(home) = dirs_path() {
                c.push(home.join("config.toml"));
            }
            c
        },
        |p| vec![p.to_path_buf()],
    );

    for path in &candidates {
        if path.exists() {
            let content =
                std::fs::read_to_string(path).map_err(|e| ConfigError::Io(path.clone(), e))?;
            let config: Config =
                toml::from_str(&content).map_err(|e| ConfigError::Parse(path.clone(), e))?;
            tracing::info!("Loaded configuration from {}", path.display());
            return Ok(config);
        }
    }

    tracing::info!("No configuration file found, using defaults");
    Ok(Config::default())
}

fn dirs_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/devserial"))
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|h| PathBuf::from(h).join("devserial"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Configuration loading errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {}: {}", .0.display(), .1)]
    Io(PathBuf, std::io::Error),
    #[error("failed to parse config file {}: {}", .0.display(), .1)]
    Parse(PathBuf, toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.global.flush_interval_ms, 100);
        assert_eq!(config.global.flush_batch_size, 1000);
        assert!(config.ports.is_empty());
        assert!(config.macros.is_empty());
    }

    #[test]
    fn test_deserialize_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.global.flush_interval_ms, 100);
        assert_eq!(config.global.flush_batch_size, 1000);
    }

    #[test]
    fn test_deserialize_partial_toml() {
        let toml_str = r"
[global]
flush_interval_ms = 50
";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.global.flush_interval_ms, 50);
        assert_eq!(config.global.flush_batch_size, 1000); // default
    }

    #[test]
    fn test_deserialize_full_config() {
        let toml_str = r#"
[global]
data_dir = "/tmp/serial"
archive_dir = "/tmp/serial/archive"
flush_interval_ms = 200
flush_batch_size = 500
log_level = "debug"

[ports."/dev/ttyUSB0"]
baudrate = 9600
data_bits = 8
parity = "none"
stop_bits = 1
flow_control = "none"
auto_reconnect = true

[ports."/dev/ttyUSB1"]
baudrate = 115200

[macros.reset_esp32]
description = "Reset ESP32 via DTR toggle"
steps = [
    { action = "dtr", value = false },
    { action = "delay", ms = 100 },
    { action = "dtr", value = true },
]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.global.flush_interval_ms, 200);
        assert_eq!(config.ports.len(), 2);
        assert_eq!(config.ports["/dev/ttyUSB0"].baudrate, 9600);
        assert_eq!(config.ports["/dev/ttyUSB1"].baudrate, 115_200);
        assert_eq!(config.macros.len(), 1);
        assert_eq!(config.macros["reset_esp32"].steps.len(), 3);
    }

    #[test]
    fn test_invalid_toml() {
        let result: Result<Config, _> = toml::from_str("invalid = [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_no_file() {
        let config = load_config(Some(Path::new("/nonexistent/path.toml"))).unwrap();
        // Falls back to default since file doesn't exist
        assert_eq!(config.global.flush_interval_ms, 100);
    }

    #[test]
    fn test_load_config_with_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(&path, "[global]\nflush_interval_ms = 42\n").unwrap();

        let config = load_config(Some(&path)).unwrap();
        assert_eq!(config.global.flush_interval_ms, 42);
    }

    #[test]
    fn test_load_config_invalid_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "not valid toml [[[").unwrap();

        let result = load_config(Some(&path));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }

    #[test]
    fn test_port_config_defaults() {
        let port = PortConfig::default();
        assert_eq!(port.baudrate, 115_200);
        assert_eq!(port.data_bits, 8);
        assert_eq!(port.parity, "none");
        assert_eq!(port.stop_bits, 1);
        assert!(port.auto_reconnect);
    }

    #[test]
    fn test_macro_step_serialization() {
        let toml_str = r#"
description = "test"
steps = [
    { action = "write", value = "AT+RST\r\n" },
    { action = "delay", ms = 500 },
    { action = "dtr", value = true },
    { action = "rts", value = false },
]
"#;
        let m: MacroConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(m.steps.len(), 4);
        assert!(matches!(&m.steps[0], MacroStep::Write { value } if value == "AT+RST\r\n"));
        assert!(matches!(&m.steps[1], MacroStep::Delay { ms: 500 }));
        assert!(matches!(&m.steps[2], MacroStep::Dtr { value: true }));
        assert!(matches!(&m.steps[3], MacroStep::Rts { value: false }));
    }
}
