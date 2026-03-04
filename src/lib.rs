pub use agtx_core::*;

// TUI-specific modules remain here
pub mod tui;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AppMode {
    Dashboard,
    Project(PathBuf),
}
