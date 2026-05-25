// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

use clap::{Parser, Subcommand};

/// `DevSerial` — Serial hardware bridge for LLMs and developers.
#[derive(Parser)]
#[command(name = "devserial", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run as MCP server (stdio transport) — this is the default
    Mcp,

    /// Open a standalone GUI serial monitor
    #[cfg(feature = "monitor")]
    Monitor {
        /// Serial port path (e.g. /dev/ttyUSB0)
        port: String,
        /// Baud rate
        #[arg(short, long, default_value = "115200")]
        baud: u32,
    },

    /// Open a TUI serial monitor in the terminal
    #[cfg(feature = "tui")]
    Tui {
        /// Serial port path (e.g. /dev/ttyUSB0)
        port: String,
        /// Baud rate
        #[arg(short, long, default_value = "115200")]
        baud: u32,
    },

    /// List available serial ports
    List,

    /// Flash firmware to an ESP device (requires espflash)
    #[cfg(feature = "esp")]
    Flash {
        /// Serial port path
        port: String,
        /// Path to firmware file (ELF or .bin)
        firmware: String,
        /// Baud rate for flashing
        #[arg(short, long)]
        baud: Option<u32>,
    },

    /// Internal: monitor subprocess (used by MCP `serial_monitor_open`)
    #[command(hide = true)]
    #[cfg(feature = "monitor")]
    MonitorSubprocess {
        #[arg(long)]
        port: String,
        #[arg(long)]
        db: String,
        #[arg(long, default_value = "")]
        info: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Command::Mcp) => run_mcp()?,

        #[cfg(feature = "monitor")]
        Some(Command::Monitor { port, baud }) => {
            devserial::standalone::run_monitor_standalone(&port, baud)?;
        }

        #[cfg(feature = "tui")]
        Some(Command::Tui { port, baud }) => {
            devserial::standalone::run_tui_standalone(&port, baud)?;
        }

        Some(Command::List) => {
            let ports = serial2_tokio::SerialPort::available_ports().unwrap_or_default();
            if ports.is_empty() {
                println!("No serial ports detected.");
            } else {
                for p in &ports {
                    println!("  {}", p.display());
                }
            }
        }

        #[cfg(feature = "esp")]
        Some(Command::Flash {
            port,
            firmware,
            baud,
        }) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let result = rt.block_on(devserial::esp::flash(&port, &firmware, baud, false));
            match result {
                Ok(output) => print!("{output}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }

        #[cfg(feature = "monitor")]
        Some(Command::MonitorSubprocess { port, db, info }) => {
            devserial::monitor::run_monitor(&port, std::path::Path::new(&db), &info)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
    }

    Ok(())
}

fn run_mcp() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            use rmcp::{ServiceExt, transport::stdio};
            use tracing_subscriber::EnvFilter;

            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
                )
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .init();

            tracing::info!("Starting DevSerial MCP Server");

            let config = devserial::config::load_config(None).unwrap_or_default();
            tracing::info!(data_dir = %config.global.data_dir.display(), "config loaded");

            let port_manager = devserial::port_manager::PortManagerHandle::new();

            // Restore previously open ports
            if let Ok(state_db) = devserial::state::StateDb::open(&config.global.data_dir) {
                if let Ok(ports) = state_db.active_ports() {
                    for entry in &ports {
                        let data_dir = config.global.data_dir.clone();
                        match port_manager
                            .open_serial(entry.name.clone(), entry.config.clone(), data_dir)
                            .await
                        {
                            Ok(()) => tracing::info!(port = %entry.name, "restored port"),
                            Err(e) => {
                                tracing::warn!(port = %entry.name, error = %e, "failed to restore port (will retry on next start)");
                            }
                        }
                    }
                }
            }

            let server = devserial::server::DevSerialServer::new(port_manager, config);
            let service = server.serve(stdio()).await.inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;

            service.waiting().await?;
            Ok(())
        })
}
