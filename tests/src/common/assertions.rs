//! Common test assertions

use std::path::Path;
use anyhow::Result;

/// Assert that a file exists and contains expected content
pub fn assert_file_contains(path: &Path, expected: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    assert!(
        content.contains(expected),
        "File {:?} does not contain expected content: {}",
        path,
        expected
    );
    Ok(())
}

/// Assert that a directory exists and contains expected files
pub fn assert_dir_contains_files(dir: &Path, expected_files: &[&str]) -> Result<()> {
    for file in expected_files {
        let path = dir.join(file);
        assert!(
            path.exists(),
            "Expected file {:?} does not exist in directory {:?}",
            file,
            dir
        );
    }
    Ok(())
}

/// Assert that a command output contains expected text
pub fn assert_output_contains(output: &std::process::Output, expected: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(
        stdout.contains(expected) || stderr.contains(expected),
        "Command output does not contain '{}'\nSTDOUT:\n{}\nSTDERR:\n{}",
        expected,
        stdout,
        stderr
    );
}