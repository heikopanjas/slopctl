//! vibe-check - A manager for coding agent instruction files
//!
//! This library provides functionality to manage, organize, and maintain
//! initialization prompts and instruction files for AI coding assistants.

mod bom;
mod config;
mod download_manager;
mod file_tracker;
mod template_engine_v1;
mod template_engine_v2;
mod template_manager;
mod utils;

pub use bom::BillOfMaterials;
pub use config::Config;
pub use download_manager::DownloadManager;
pub use file_tracker::{FileMetadata, FileStatus, FileTracker};
pub use template_engine_v1::TemplateEngineV1;
pub use template_engine_v2::TemplateEngineV2;
pub use template_manager::TemplateManager;
pub use utils::{FileActionResponse, confirm_action, copy_dir_all, copy_file_with_mkdir, prompt_file_modification, remove_file_and_cleanup_parents};

/// Result type used throughout the library
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
