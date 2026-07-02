use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::Command;

pub fn tail_log_lines(path: &Path, lines: usize) -> AppResult<String> {
    let output = Command::new("tail")
        .arg("-n")
        .arg(lines.to_string())
        .arg(path)
        .output()
        .map_err(|source| AppError::CommandIo {
            program: "tail".to_string(),
            source,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(format!("{stdout}{stderr}").trim_end().to_string())
}
