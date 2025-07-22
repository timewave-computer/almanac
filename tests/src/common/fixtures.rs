//! Common fixture loading utilities

use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::de::DeserializeOwned;

/// Load a JSON fixture file
pub fn load_json_fixture<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Load a TOML fixture file
pub fn load_toml_fixture<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

/// Get the fixtures directory path
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Get a specific fixture file path
pub fn fixture_path(relative_path: &str) -> PathBuf {
    fixtures_dir().join(relative_path)
}