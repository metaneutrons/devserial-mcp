// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

pub mod config;
#[cfg(feature = "esp")]
pub mod esp;
#[cfg(feature = "monitor")]
pub mod monitor;
pub mod port_manager;
pub mod reader;
pub mod server;
pub mod standalone;
pub mod state;
pub mod storage;
#[cfg(feature = "tui")]
pub mod tui;

#[cfg(any(test, feature = "testutil"))]
pub mod testutil;
