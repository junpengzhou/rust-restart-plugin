use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn combined_output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    pub fn success(&self) -> bool {
        self.code == 0
    }
}

pub trait CommandRunner {
    fn run_capture(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> AppResult<CommandOutput>;

    fn run_inherit(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> AppResult<CommandOutput>;
}

pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run_capture(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> AppResult<CommandOutput> {
        let mut command = Command::new(program);
        command.args(args);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }

        let output = command.output().map_err(|source| AppError::CommandIo {
            program: program.to_string(),
            source,
        })?;

        Ok(CommandOutput {
            code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    fn run_inherit(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> AppResult<CommandOutput> {
        let mut command = Command::new(program);
        command.args(args);
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }

        let status = command.status().map_err(|source| AppError::CommandIo {
            program: program.to_string(),
            source,
        })?;

        Ok(CommandOutput {
            code: status.code().unwrap_or(1),
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}
