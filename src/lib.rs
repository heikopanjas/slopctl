//! vibe-check - A manager for coding agent instruction files
//!
//! This library provides functionality to manage, organize, and maintain
//! initialization prompts and instruction files for AI coding assistants.

mod bom;
mod download_manager;
mod template_manager;
mod utils;

pub use bom::BillOfMaterials;
pub use download_manager::DownloadManager;
pub use template_manager::TemplateManager;
pub use utils::{copy_dir_all, copy_file_with_mkdir, remove_file_and_cleanup_parents};

/// Result type used throughout the library
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
