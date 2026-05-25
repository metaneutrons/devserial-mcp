# devserial

[![CI](https://github.com/metaneutrons/devserial-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/metaneutrons/devserial-mcp/actions/workflows/ci.yml)
[![Release](https://github.com/metaneutrons/devserial-mcp/actions/workflows/release.yml/badge.svg)](https://github.com/metaneutrons/devserial-mcp/actions/workflows/release.yml)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

MCP server and standalone tool bridging serial hardware to LLMs via SQLite-backed buffer.

## Features

- **MCP server** — stdio transport, works with Claude Desktop, Cursor, and any MCP client
- **Standalone GUI** — native monitor window with egui (optional `monitor` feature)
- **Standalone TUI** — terminal monitor with ratatui (optional `tui` feature)
- **Persistent buffering** — all serial data stored in per-port SQLite databases (WAL mode)
- **Grep-like search** — substring, exact, and regex queries with optional time bounds
- **Export** — txt, csv (RFC 4180), and jsonl formats
- **Hardware control** — DTR/RTS signals, macro sequences (reset, bootloader entry)
- **ESP tooling** — flash, erase, board-info via espflash subprocess (optional `esp` feature)
- **Auto-reconnect** — exponential backoff on device disconnect
- **User-defined macros** — TOML config with custom DTR/RTS/write/delay sequences

## Installation

```bash
cargo install --path . --all-features
```

## Usage

### MCP Server (default)

```bash
devserial          # starts MCP server on stdio
```

### Standalone Modes

```bash
devserial list                           # list available serial ports
devserial monitor /dev/ttyUSB0 -b 115200 # GUI monitor window
devserial tui /dev/ttyUSB0 -b 115200     # TUI in terminal
devserial flash /dev/ttyUSB0 firmware.elf # flash ESP device
```

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "serial": {
      "command": "/path/to/devserial"
    }
  }
}
```

## Configuration

Auto-discovered: `./devserial.toml` or `~/.config/devserial/config.toml`

```toml
[global]
data_dir = "./data"

[ports."/dev/ttyUSB0"]
baudrate = 115200
auto_reconnect = true
max_buffer_lines = 100000

[macros.reset_esp32]
description = "Reset ESP32 via DTR toggle"
steps = [
    { action = "dtr", value = false },
    { action = "delay", ms = 100 },
    { action = "dtr", value = true },
]
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `serial_read` | Read captured data (supports negative start_line for tail) |
| `serial_status` | Port status: connection state, lines, bytes, last activity |
| `serial_search` | Grep-like search (substring/exact/regex) with time bounds |
| `serial_export` | Export to txt/csv/jsonl file |
| `serial_clear` | Clear captured data with optional archive |
| `serial_list` | List system ports and managed connections |
| `serial_open` | Open a serial port (uses TOML config as defaults) |
| `serial_close` | Close a managed serial port |
| `serial_write` | Write data (UTF-8 or hex with 0x prefix) |
| `serial_signal` | Set DTR/RTS signals |
| `serial_macro` | Run macro sequences (built-in + user-defined) |
| `serial_monitor_open` | Open native GUI monitor window |
| `serial_monitor_close` | Close the monitor window |
| `serial_esp_flash` | Flash firmware to ESP device |
| `serial_esp_info` | Get ESP chip/board info |
| `serial_esp_erase` | Erase entire ESP flash |
| `serial_esp_write_bin` | Write raw binary to flash address |

## Features (Cargo)

| Feature | Default | Description |
|---------|---------|-------------|
| `esp` | ✅ | ESP tooling via espflash subprocess |
| `monitor` | ❌ | Native GUI window (egui) |
| `tui` | ❌ | Terminal UI (ratatui) |

## Development

Requires Rust 1.85+ (edition 2024).

```bash
just setup    # configure git hooks
just check    # fmt + clippy + test
just test     # run all tests
just fmt      # format code
just release  # build release
```

## License

GPL-3.0-only — see [LICENSE](LICENSE).
