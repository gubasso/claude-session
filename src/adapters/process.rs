//! Replaceable process spawning and waiting boundary.

use std::{
    io,
    os::unix::process::ExitStatusExt,
    process::{Command, Stdio},
};

use crate::{
    domain::child::{CapturedChild, ChildInvocation, ChildOutcome},
    error::{AppError, Diagnostic},
};

/// Process operations needed by passthrough and read-only subroutines.
pub(crate) trait ProcessRunner {
    /// Spawns with inherited streams and waits for the exact child.
    fn run_inherited(&self, invocation: &ChildInvocation) -> Result<ChildOutcome, AppError>;
    /// Runs a read-only child subroutine and captures its bytes.
    fn run_captured(&self, invocation: &ChildInvocation) -> Result<CapturedChild, AppError>;
}

/// Real Linux process runner.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemProcessRunner;

fn command(invocation: &ChildInvocation) -> Command {
    let mut command = Command::new(invocation.program());
    command.args(invocation.arguments()).env_clear();
    command.envs(invocation.environment().iter().cloned());
    command
}

fn outcome(status: std::process::ExitStatus) -> ChildOutcome {
    status.code().map_or_else(
        || ChildOutcome::Signaled(status.signal().unwrap_or(0)),
        |code| ChildOutcome::Exited(u8::try_from(code).unwrap_or(255)),
    )
}

fn spawn_error(error: &io::Error, program: &std::path::Path) -> AppError {
    match error.raw_os_error() {
        Some(2) => AppError::child_not_found(program.display().to_string(), error.to_string()),
        Some(1 | 8 | 13) => {
            AppError::child_not_executable(program.display().to_string(), error.to_string())
        }
        _ => AppError::new(
            crate::error::ErrorKind::OsError,
            Diagnostic::new(
                "child process could not be spawned",
                program.display().to_string(),
                error.to_string(),
                "inspect the operating-system error",
            ),
        ),
    }
}

impl ProcessRunner for SystemProcessRunner {
    fn run_inherited(&self, invocation: &ChildInvocation) -> Result<ChildOutcome, AppError> {
        let mut command = command(invocation);
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let mut child = command
            .spawn()
            .map_err(|error| spawn_error(&error, invocation.program()))?;
        child.wait().map(outcome).map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::OsError,
                Diagnostic::new(
                    "waiting for child failed",
                    invocation.program().display().to_string(),
                    error.to_string(),
                    "retry the invocation",
                ),
            )
        })
    }
    fn run_captured(&self, invocation: &ChildInvocation) -> Result<CapturedChild, AppError> {
        let output = command(invocation)
            .output()
            .map_err(|error| spawn_error(&error, invocation.program()))?;
        Ok(CapturedChild {
            stdout: output.stdout,
            stderr: output.stderr,
            outcome: outcome(output.status),
        })
    }
}
