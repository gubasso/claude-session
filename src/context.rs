//! The single immutable application context.

use crate::{
    adapters::{
        environment::{Environment, SystemEnvironment},
        filesystem::SystemFileSystem,
        process::SystemProcessRunner,
    },
    commands::dispatch::OutputMode,
    domain::{config::ResolvedConfig, paths::XdgPaths},
    ui::writer::{Color, OutputWriter},
};

/// Concrete adapter bundle shared by handlers.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Adapters {
    filesystem: SystemFileSystem,
    process: SystemProcessRunner,
}

impl Adapters {
    /// Returns the filesystem adapter.
    pub(crate) const fn filesystem(self) -> SystemFileSystem {
        self.filesystem
    }
    /// Returns the process adapter.
    pub(crate) const fn process(self) -> SystemProcessRunner {
        self.process
    }
}

/// Immutable state for one invocation.
pub(crate) struct AppContext {
    config: ResolvedConfig,
    paths: XdgPaths,
    environment: SystemEnvironment,
    output_mode: OutputMode,
    writer: OutputWriter,
    color: Color,
    adapters: Adapters,
}

impl AppContext {
    /// Constructs the sole context after bootstrap resolution.
    pub(crate) fn new(
        config: ResolvedConfig,
        paths: XdgPaths,
        environment: SystemEnvironment,
        output_mode: OutputMode,
    ) -> Self {
        let writer = OutputWriter::system();
        // The one place the ladder is read. Every renderer takes the answer
        // from here, which is what makes two surfaces in one invocation
        // impossible to disagree.
        let color = Color::resolve(
            environment.variables(),
            output_mode,
            writer.stdout_is_terminal(),
            writer.stderr_is_terminal(),
        );
        Self {
            config,
            paths,
            environment,
            output_mode,
            writer,
            color,
            adapters: Adapters::default(),
        }
    }
    /// Returns resolved configuration.
    pub(crate) const fn config(&self) -> &ResolvedConfig {
        &self.config
    }
    /// Returns resolved XDG paths.
    // Logging resolves its own namespace at the entry point, before a context
    // exists. The first downstream reader is the account and profile storage.
    #[allow(dead_code, reason = "no command writes to a namespace yet")]
    pub(crate) const fn paths(&self) -> &XdgPaths {
        &self.paths
    }
    /// Returns the environment snapshot.
    pub(crate) const fn environment(&self) -> &SystemEnvironment {
        &self.environment
    }
    /// Returns the active output mode.
    pub(crate) const fn output_mode(&self) -> OutputMode {
        self.output_mode
    }
    /// Returns the sole output writer.
    pub(crate) const fn writer(&self) -> &OutputWriter {
        &self.writer
    }
    /// Returns the invocation's color decision.
    // Resolved here so the first renderer to colour a named surface reads an
    // answer rather than deriving one. No surface is coloured yet.
    #[allow(dead_code, reason = "no renderer applies the decision yet")]
    pub(crate) const fn color(&self) -> Color {
        self.color
    }
    /// Returns concrete adapters.
    pub(crate) const fn adapters(&self) -> &Adapters {
        &self.adapters
    }
}
