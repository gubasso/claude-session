//! Best-effort single-subscriber logging bootstrap.

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    thread::JoinHandle,
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

use crate::{
    adapters::filesystem::SystemFileSystem,
    commands::dispatch::{OutputMode, Verbosity},
    domain::paths::XdgPaths,
    ui::writer::OutputWriter,
};

/// Owns the non-blocking worker until the boundary drops it.
pub(crate) struct LoggingGuard {
    sender: Option<SyncSender<Option<String>>>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for LoggingGuard {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(None);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct FileSubscriber {
    sender: Mutex<SyncSender<Option<String>>>,
    terminal: Option<Level>,
    /// Whether the mirror decorates its level word, resolved once by the same
    /// ladder every other surface reads.
    color: bool,
    next_span: AtomicU64,
}

impl Subscriber for FileSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= Level::DEBUG
            || self
                .terminal
                .is_some_and(|level| *metadata.level() <= level)
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(self.next_span.fetch_add(1, Ordering::Relaxed))
    }
    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut visitor = Fields::default();
        event.record(&mut visitor);
        let message = visitor.values.remove("message").unwrap_or_default();
        let sentence = std::mem::take(&mut visitor.message);
        let status = visitor.take("status");
        let duration = visitor.take("dur_ms");
        let error_kind = visitor.take("err.kind");
        // An event a renderer is already saying in full opts out of the mirror
        // rather than being said twice in two shapes.
        let mirrored = visitor.values.remove("mirror").as_deref() != Some("false");
        let mut line = format!(
            "ts={} level={} target={} op={} msg={}",
            timestamp(),
            metadata.level().as_str().to_ascii_lowercase(),
            metadata.target(),
            visitor.take("op"),
            message
        );
        append_optional(&mut line, "status", &status);
        append_optional(&mut line, "dur_ms", &duration);
        append_optional(&mut line, "err.kind", &error_kind);
        for (key, value) in visitor.values {
            line.push(' ');
            line.push_str(&key);
            line.push('=');
            line.push_str(&value);
        }
        line.push('\n');
        if *metadata.level() <= Level::DEBUG
            && let Ok(sender) = self.sender.lock()
        {
            let _ = sender.try_send(Some(line.clone()));
        }
        // The file keeps the key-value record; the terminal gets the sentence.
        // One event, two audiences, and the machine one is the reason the human
        // one can drop every field ([ADR-0093]).
        if mirrored
            && self
                .terminal
                .is_some_and(|level| *metadata.level() <= level)
        {
            let _ = OutputWriter::system()
                .stderr(mirror(self.color, *metadata.level(), &sentence).as_bytes());
        }
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(if self.terminal == Some(Level::TRACE) {
            tracing::metadata::LevelFilter::TRACE
        } else {
            tracing::metadata::LevelFilter::DEBUG
        })
    }
}

fn append_optional(line: &mut String, key: &str, value: &str) {
    if !value.is_empty() {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(value);
    }
}

#[derive(Default)]
struct Fields {
    values: BTreeMap<String, String>,
    /// The message exactly as it was written, before the key-value form
    /// escapes and quotes it. The file needs the escaped one and the terminal
    /// needs this one, so both are kept rather than one being recovered.
    message: String,
}

impl Fields {
    fn take(&mut self, key: &str) -> String {
        self.values.remove(key).unwrap_or_default()
    }
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        // Only the delimiters `Debug` added for a string are removed, not every
        // quote the value itself carries.
        let unquoted = rendered
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(&rendered);
        if field.name() == "message" {
            unquoted.clone_into(&mut self.message);
        }
        self.values
            .insert(field.name().to_owned(), escape(unquoted));
    }
}

/// Keeps one record on one line with parsable `key=value` pairs.
///
/// A record carries a path or a message, and either may hold a space or a
/// newline. Unescaped, the first breaks field parsing and the second breaks the
/// one-line contract the whole format rests on.
fn escape(value: &str) -> String {
    if !value.contains([' ', '\n', '\r', '\t', '"', '\\']) {
        return value.to_owned();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(character),
        }
    }
    out.push('"');
    out
}

/// Installs exactly one subscriber, returning the guard the boundary flushes by dropping.
pub(crate) fn install(
    paths: &XdgPaths,
    verbosity: Verbosity,
    mode: OutputMode,
    environment: &[(OsString, OsString)],
) -> LoggingGuard {
    let empty = || LoggingGuard {
        sender: None,
        worker: None,
    };
    if let Err(error) = SystemFileSystem::create_private_dir(paths.state()) {
        report_unavailable(&error);
        return empty();
    }
    let log_path = paths.state().join("claude-session.log");
    rotate(&log_path);
    let mut file = match SystemFileSystem::open_private_log(&log_path) {
        Ok(file) => file,
        Err(error) => {
            report_unavailable(&error);
            return empty();
        }
    };
    let (sender, receiver) = sync_channel::<Option<String>>(1024);
    let worker = std::thread::spawn(move || {
        while let Ok(Some(line)) = receiver.recv() {
            if let Err(error) = file.write_all(line.as_bytes()) {
                report_unavailable(&error);
                break;
            }
        }
        if let Err(error) = file.flush() {
            report_unavailable(&error);
        }
    });
    let terminal = terminal_level(verbosity, mode, environment);
    let writer = OutputWriter::system();
    let subscriber = FileSubscriber {
        sender: Mutex::new(sender.clone()),
        terminal,
        color: crate::ui::writer::Color::resolve(
            environment,
            mode,
            writer.stdout_is_terminal(),
            writer.stderr_is_terminal(),
        )
        .stderr(),
        next_span: AtomicU64::new(1),
    };
    if let Err(error) = tracing::subscriber::set_global_default(subscriber) {
        report_unavailable(&std::io::Error::other(error));
        return empty();
    }
    LoggingGuard {
        sender: Some(sender),
        worker: Some(worker),
    }
}

fn report_unavailable(error: &std::io::Error) {
    // The one warning raised before a subscriber exists, so it cannot go
    // through the mirror and instead borrows its exact shape.
    let _ = OutputWriter::system().stderr(
        mirror(
            false,
            Level::WARN,
            &format!(
                concat!(
                    "this run is not being logged, because its log file could not",
                    " be opened: {}. Everything else works; only the record of",
                    " what happened is missing"
                ),
                error
            ),
        )
        .as_bytes(),
    );
}

/// The stderr mirror's one line: a level word, then a sentence.
///
/// Wrapped and indented like every other human surface, and carrying no field
/// the log file does not already have.
fn mirror(color: bool, level: Level, message: &str) -> String {
    let word = level.as_str().to_ascii_lowercase();
    let decorated = if color {
        let code = match level {
            Level::ERROR => "31",
            Level::WARN => "33",
            _ => "2",
        };
        format!("\u{1b}[{code}m{word}\u{1b}[0m")
    } else {
        word.clone()
    };
    let prefix = format!("claude-session: {word}: ");
    let text = crate::ui::prose::wrap(message, &prefix, "  ");
    if color {
        text.replacen(&prefix, &format!("claude-session: {decorated}: "), 1)
    } else {
        text
    }
}

/// Resolves the level of the stderr mirror, which is a human channel.
///
/// In JSON mode standard error carries the error document, so a mirrored
/// key-value record would corrupt the one document shape a verb does not
/// choose. `RUST_LOG` is the developer's override and outranks the flags.
fn terminal_level(
    verbosity: Verbosity,
    mode: OutputMode,
    environment: &[(OsString, OsString)],
) -> Option<Level> {
    if mode == OutputMode::Json {
        return None;
    }
    let rust_log = crate::adapters::environment::value(environment, "RUST_LOG")
        .and_then(std::ffi::OsStr::to_str);
    if let Some(value) = rust_log {
        let lower = value.to_ascii_lowercase();
        if lower.contains("trace") {
            return Some(Level::TRACE);
        }
        if lower.contains("debug") {
            return Some(Level::DEBUG);
        }
        if lower.contains("info") {
            return Some(Level::INFO);
        }
        if lower.contains("warn") {
            return Some(Level::WARN);
        }
        if lower.contains("error") {
            return Some(Level::ERROR);
        }
    }
    // The default mirrors warnings and above. Mirroring nothing would make the
    // wrapper silent about its own recoverable conditions in exactly the run a
    // user makes without flags.
    Some(match verbosity {
        Verbosity::Quiet => Level::ERROR,
        Verbosity::Default => Level::WARN,
        Verbosity::Info => Level::INFO,
        Verbosity::Debug => Level::DEBUG,
        Verbosity::Trace => Level::TRACE,
    })
}

fn timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        day_seconds / 3600,
        day_seconds / 60 % 60,
        day_seconds % 60,
        duration.subsec_millis()
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn rotate(path: &Path) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.len() < 8 * 1024 * 1024 {
        return;
    }
    for index in (1..=3).rev() {
        let from = if index == 1 {
            path.to_path_buf()
        } else {
            path.with_extension(format!("log.{}", index - 1))
        };
        let to = path.with_extension(format!("log.{index}"));
        let _ = fs::rename(from, to);
    }
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn a_value_without_separators_is_left_alone() {
        assert_eq!(escape("resolve_child"), "resolve_child");
        assert_eq!(escape("/usr/bin/claude"), "/usr/bin/claude");
        assert_eq!(escape(""), "");
    }

    #[test]
    fn a_separator_forces_quoting_so_fields_stay_parsable() {
        assert_eq!(escape("two words"), "\"two words\"");
        assert_eq!(escape("/a path/claude"), "\"/a path/claude\"");
    }

    #[test]
    fn a_newline_never_reaches_the_record() {
        assert_eq!(escape("a\nb"), "\"a\\nb\"");
        assert_eq!(escape("a\r\nb"), "\"a\\r\\nb\"");
        assert!(!escape("a\nb").contains('\n'));
    }

    #[test]
    fn a_quote_or_backslash_is_escaped_rather_than_dropped() {
        assert_eq!(escape("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(escape("a\\b c"), "\"a\\\\b c\"");
    }
}
