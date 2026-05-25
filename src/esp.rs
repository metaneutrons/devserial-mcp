// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

//! ESP tooling via `espflash` subprocess.

use std::sync::OnceLock;

use tokio::process::Command;

/// Cached path to the espflash binary (None if not found).
static ESPFLASH_PATH: OnceLock<Option<String>> = OnceLock::new();

/// Check if espflash is available in $PATH. Caches the result.
#[must_use]
pub fn is_available() -> bool {
    ESPFLASH_PATH
        .get_or_init(|| detect_espflash().ok())
        .is_some()
}

fn detect_espflash() -> Result<String, std::io::Error> {
    let output = std::process::Command::new("espflash")
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!(version = %version, "espflash detected");
        Ok("espflash".to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "espflash not found",
        ))
    }
}

fn espflash_bin() -> &'static str {
    ESPFLASH_PATH
        .get()
        .and_then(Option::as_deref)
        .unwrap_or("espflash")
}

/// Flash firmware to an ESP device.
///
/// # Errors
/// Returns error if espflash fails or is not available.
pub async fn flash(
    port: &str,
    firmware_path: &str,
    baud: Option<u32>,
    monitor_after: bool,
) -> Result<String, String> {
    let mut cmd = Command::new(espflash_bin());
    cmd.arg("flash").arg("--port").arg(port);

    if let Some(b) = baud {
        cmd.arg("--baud").arg(b.to_string());
    }

    if monitor_after {
        cmd.arg("--monitor");
    }

    cmd.arg(firmware_path);

    run_command(cmd).await
}

/// Get board/chip information.
///
/// # Errors
/// Returns error if espflash fails or is not available.
pub async fn board_info(port: &str) -> Result<String, String> {
    let mut cmd = Command::new(espflash_bin());
    cmd.arg("board-info").arg("--port").arg(port);
    run_command(cmd).await
}

/// Erase entire flash.
///
/// # Errors
/// Returns error if espflash fails or is not available.
pub async fn erase_flash(port: &str) -> Result<String, String> {
    let mut cmd = Command::new(espflash_bin());
    cmd.arg("erase-flash").arg("--port").arg(port);
    run_command(cmd).await
}

/// Write a binary file to a specific flash address.
///
/// # Errors
/// Returns error if espflash fails or is not available.
pub async fn write_bin(port: &str, file_path: &str, address: &str) -> Result<String, String> {
    let mut cmd = Command::new(espflash_bin());
    cmd.arg("write-bin")
        .arg("--port")
        .arg(port)
        .arg(address)
        .arg(file_path);
    run_command(cmd).await
}

async fn run_command(mut cmd: Command) -> Result<String, String> {
    let output = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to run espflash: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        let mut result = stdout.into_owned();
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&stderr);
        }
        Ok(result)
    } else {
        let mut err = stderr.into_owned();
        if err.is_empty() {
            err = stdout.into_owned();
        }
        if err.is_empty() {
            err = format!("espflash exited with status {}", output.status);
        }
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_caches_result() {
        // Call twice — should return same result (cached via OnceLock)
        let first = is_available();
        let second = is_available();
        assert_eq!(first, second);
    }

    #[test]
    fn test_espflash_bin_returns_espflash() {
        // Even if not available, the binary name should be "espflash"
        assert_eq!(espflash_bin(), "espflash");
    }

    #[tokio::test]
    async fn test_flash_nonexistent_firmware() {
        if !is_available() {
            return; // Skip if espflash not installed
        }
        let result = flash(
            "/dev/nonexistent_port_xyz",
            "/nonexistent/firmware.elf",
            None,
            false,
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_board_info_nonexistent_port() {
        if !is_available() {
            return;
        }
        let result = board_info("/dev/nonexistent_port_xyz").await;
        assert!(result.is_err());
    }
}
