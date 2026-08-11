//! Replaceable process launching boundary.

use std::{
    io,
    os::unix::process::{CommandExt as _, ExitStatusExt},
    process::{Command, Stdio},
};

use crate::{
    domain::child::{CapturedChild, ChildInvocation, ChildOutcome},
    error::{AppError, Diagnostic},
};

/// Process operations needed by passthrough and read-only subroutines.
pub(crate) trait ProcessRunner {
    /// Replaces this process with the child, returning only on failure.
    ///
    /// There is no success value to return: on success this call does not come
    /// back, because the image running afterwards is the child's.
    fn exec(&self, invocation: &ChildInvocation) -> AppError;
    /// Runs a child subroutine on the inherited streams and waits for it.
    ///
    /// This is the composed-verb case, not a launch: the wrapper asked the child
    /// a question, keeps its own exit code, and therefore has to survive the
    /// answer ([ADR-0068](../../docs/decisions/ADR-0068-spawn-the-child-as-a-subroutine.md)).
    fn run_inherited(&self, invocation: &ChildInvocation) -> Result<ChildOutcome, AppError>;
    /// Runs an interactive child while reserving wrapper stdout for JSON.
    fn run_stdout_to_stderr(&self, invocation: &ChildInvocation) -> Result<ChildOutcome, AppError>;
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

fn launch_error(error: &io::Error, program: &std::path::Path) -> AppError {
    match error.raw_os_error() {
        Some(2) => AppError::child_not_found(program.display().to_string(), error.to_string()),
        Some(1 | 8 | 13) => {
            AppError::child_not_executable(program.display().to_string(), error.to_string())
        }
        _ => AppError::new(
            crate::error::ErrorKind::OsError,
            Diagnostic::new(
                "the child process could not be launched",
                program.display().to_string(),
                error.to_string(),
                "inspect the operating-system error",
            ),
        ),
    }
}

impl ProcessRunner for SystemProcessRunner {
    fn exec(&self, invocation: &ChildInvocation) -> AppError {
        // Nothing to inherit explicitly and no streams to wire: an image
        // replacement keeps the descriptors, the working directory, and the
        // process id it was called with. That is the whole reason the wrapper
        // owes no supervision.
        let error = command(invocation).exec();
        launch_error(&error, invocation.program())
    }
    fn run_inherited(&self, invocation: &ChildInvocation) -> Result<ChildOutcome, AppError> {
        let mut child = command(invocation)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| launch_error(&error, invocation.program()))?;
        child.wait().map(outcome).map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::OsError,
                Diagnostic::new(
                    "waiting for the child subroutine failed",
                    invocation.program().display().to_string(),
                    error.to_string(),
                    "retry the invocation",
                ),
            )
        })
    }
    fn run_stdout_to_stderr(&self, invocation: &ChildInvocation) -> Result<ChildOutcome, AppError> {
        let stderr = io::stderr();
        let mut child = command(invocation)
            .stdin(Stdio::inherit())
            .stdout(Stdio::from(stderr))
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| launch_error(&error, invocation.program()))?;
        child.wait().map(outcome).map_err(|error| {
            AppError::new(
                crate::error::ErrorKind::OsError,
                Diagnostic::new(
                    "waiting for the child subroutine failed",
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
            .map_err(|error| launch_error(&error, invocation.program()))?;
        Ok(CapturedChild {
            stdout: output.stdout,
            stderr: output.stderr,
            outcome: outcome(output.status),
        })
    }
}
