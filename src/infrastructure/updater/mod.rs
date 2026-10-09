pub mod cache;
pub mod client;
pub mod installer;
pub mod types;

pub use cache::{cleanup_stale_update_files, get_cached_update, spawn_background_update_check};
pub use client::check_and_save_latest_version;
pub use installer::run_upgrade;
pub use types::{is_newer_version, UpdateInfo};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "v1.1.0"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(is_newer_version("1.2.3", "1.3.0"));

        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "v1.0.0"));
        assert!(!is_newer_version("1.1.0", "1.0.5"));
        assert!(!is_newer_version("2.0.0", "1.9.9"));
    }
}
