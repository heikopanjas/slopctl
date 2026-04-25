//! slopctl - A manager for coding agent instruction files
//!
//! This library provides functionality to manage, organize, and maintain
//! initialization prompts and instruction files for AI coding assistants.

/// Early-return guard macro for precondition checks
///
/// Returns the given expression when the condition is false.
/// Works with any return type: `Result`, `Option`, or bare values.
///
/// # Examples
///
/// ```rust,ignore
/// require!(path.exists() == true, Err(anyhow::anyhow!("File not found")));
/// require!(count > 0, None);
/// require!(input.is_empty() == false, Ok(()));
/// ```
#[macro_export]
macro_rules! require {
    ($cond:expr, $ret:expr) => {
        if ($cond) == false
        {
            return $ret;
        }
    };
}

pub mod agent_defaults;
mod bom;
pub mod cli;
mod config;
mod download_manager;
mod file_tracker;
pub mod github;
pub mod llm;
mod template_engine;
mod template_manager;
mod utils;

pub use anyhow::Result;
pub use bom::BillOfMaterials;
pub use config::{Config, ConfigScope, EffectiveConfig};
pub use download_manager::DownloadManager;
pub use file_tracker::{AGENT_ALL, FileMetadata, FileStatus, FileTracker, LANG_NONE, SLOPCTL_DIR, legacy_tracker_path};
pub use template_engine::{ResolvedContent, ResolvedFile, ResolvedFiles, TemplateContext, TemplateEngine, UpdateOptions, normalize_path};
pub use template_manager::{MergeOptions, TemplateManager};
pub use utils::{
    FileActionResponse, collect_files_recursive, confirm_action, copy_dir_all, copy_file_with_mkdir, prompt_file_modification, remove_file_and_cleanup_parents
};

/// Serializes tests that modify environment variables (process-global state).
/// Shared across all test modules in the crate.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests
{
    #[test]
    fn test_require_passes_when_true()
    {
        fn check(val: bool) -> Option<&'static str>
        {
            require!(val == true, None);
            Some("ok")
        }
        assert_eq!(check(true), Some("ok"));
    }

    #[test]
    fn test_require_returns_when_false()
    {
        fn check(val: bool) -> Option<&'static str>
        {
            require!(val == true, None);
            Some("ok")
        }
        assert_eq!(check(false), None);
    }

    #[test]
    fn test_require_with_result() -> anyhow::Result<()>
    {
        fn validate(name: &str) -> crate::Result<()>
        {
            require!(name.is_empty() == false, Err(anyhow::anyhow!("name must not be empty")));
            Ok(())
        }
        assert!(validate("hello").is_ok() == true);
        assert!(validate("").is_err() == true);
        Ok(())
    }
}
