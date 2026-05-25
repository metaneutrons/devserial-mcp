// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

//! Standalone mode entry points (no MCP server needed).

#[cfg(any(feature = "monitor", feature = "tui"))]
use crate::config::PortConfig;

/// Open a serial port directly and launch the GUI monitor.
///
/// # Errors
/// Returns error if the port cannot be opened or the window fails.
#[cfg(feature = "monitor")]
pub fn run_monitor_standalone(port: &str, baud: u32) -> Result<(), Box<dyn std::error::Error>> {
    let config = PortConfig {
        baudrate: baud,
        ..PortConfig::default()
    };

    // Create a temp DB for the standalone session
    let data_dir = state_dir();
    std::fs::create_dir_all(&data_dir)?;
    let sanitized = port.replace(['/', '\\'], "_");
    let db_path = data_dir.join(format!("{sanitized}.db"));

    let storage = crate::storage::SqliteStorage::open(&db_path)?;

    // Spawn a reader in a background thread to feed the DB
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let storage_arc = std::sync::Arc::new(std::sync::Mutex::new(storage));
    let port_name = port.to_string();
    let config_clone = config;
    let storage_clone = std::sync::Arc::clone(&storage_arc);

    rt.spawn(async move {
        match crate::port_manager::open_serial_port_raw(&port_name, &config_clone) {
            Ok(serial_port) => {
                crate::reader::spawn_reader(serial_port, &storage_clone, &config_clone);
            }
            Err(e) => {
                eprintln!("Failed to open {port_name}: {e}");
            }
        }
        // Keep runtime alive
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });

    let info = format!("{baud} 8N1");

    // Open a sync serial port for direct writes from the monitor GUI
    let write_port = serial2::SerialPort::open(port, |mut settings: serial2::Settings| {
        settings.set_baud_rate(baud)?;
        Ok(settings)
    })?;

    // Try GUI first, fall back to TUI if no display available
    match crate::monitor::run_monitor_with_port(port, &db_path, &info, Box::new(write_port)) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("GUI failed ({e}), falling back to TUI...");
            #[cfg(feature = "tui")]
            {
                let write_handle = std::sync::Arc::new(tokio::sync::Mutex::new(
                    crate::port_manager::open_serial_port_raw(
                        port,
                        &PortConfig {
                            baudrate: baud,
                            ..PortConfig::default()
                        },
                    )?,
                ));
                crate::tui::run_tui(port, baud, &storage_arc, &write_handle, &rt)
            }
            #[cfg(not(feature = "tui"))]
            Err(e.into())
        }
    }
}

/// Open a serial port directly and launch the TUI monitor.
///
/// # Errors
/// Returns error if the port cannot be opened or the TUI fails.
#[cfg(feature = "tui")]
pub fn run_tui_standalone(port: &str, baud: u32) -> Result<(), Box<dyn std::error::Error>> {
    let config = PortConfig {
        baudrate: baud,
        ..PortConfig::default()
    };

    let data_dir = state_dir();
    std::fs::create_dir_all(&data_dir)?;
    let sanitized = port.replace(['/', '\\'], "_");
    let db_path = data_dir.join(format!("{sanitized}.db"));

    let storage = crate::storage::SqliteStorage::open(&db_path)?;
    let storage_arc = std::sync::Arc::new(std::sync::Mutex::new(storage));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // Open port and spawn reader
    let serial_port = crate::port_manager::open_serial_port_raw(port, &config)?;
    let write_port = crate::port_manager::open_serial_port_raw(port, &config)?;
    let write_handle = std::sync::Arc::new(tokio::sync::Mutex::new(write_port));

    rt.spawn({
        let storage_clone = std::sync::Arc::clone(&storage_arc);
        async move {
            crate::reader::spawn_reader(serial_port, &storage_clone, &config);
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        }
    });

    crate::tui::run_tui(port, baud, &storage_arc, &write_handle, &rt)
}

/// Platform-specific state directory.
#[cfg(any(feature = "monitor", feature = "tui"))]
fn state_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map_or_else(
            || std::path::PathBuf::from("./data"),
            |h| std::path::PathBuf::from(h).join("Library/Application Support/devserial"),
        )
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_STATE_HOME")
            .map(|h| std::path::PathBuf::from(h).join("devserial"))
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".local/state/devserial"))
            })
            .unwrap_or_else(|| std::path::PathBuf::from("./data"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|h| std::path::PathBuf::from(h).join("devserial"))
            .unwrap_or_else(|| std::path::PathBuf::from("./data"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        std::path::PathBuf::from("./data")
    }
}
