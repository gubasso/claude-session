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

use crate::{adapters::filesystem::FileSystem, domain::paths::XdgPaths, ui::writer::OutputWriter};

/// Owns the non-blocking worker until post-flight cleanup.
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
        let status = visitor.take("status");
        let duration = visitor.take("dur_ms");
        let error_kind = visitor.take("err.kind");
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
        if self
            .terminal
            .is_some_and(|level| *metadata.level() <= level)
        {
            let _ = OutputWriter::system().stderr(line.as_bytes());
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
}

impl Fields {
    fn take(&mut self, key: &str) -> String {
        self.values.remove(key).unwrap_or_default()
    }
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        self.values.insert(
            field.name().to_owned(),
            rendered.trim_matches('"').to_owned(),
        );
    }
}

/// Installs exactly one subscriber, returning a guard for post-flight flush.
pub(crate) fn install(
    paths: &XdgPaths,
    verbosity: u8,
    environment: &[(OsString, OsString)],
) -> LoggingGuard {
    let empty = || LoggingGuard {
        sender: None,
        worker: None,
    };
    if let Err(error) = FileSystem::create_private_dir(paths.state()) {
        report_unavailable(&error);
        return empty();
    }
    let log_path = paths.state().join("claude-session.log");
    rotate(&log_path);
    let mut file = match FileSystem::open_private_log(&log_path) {
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
    let terminal = terminal_level(verbosity, environment);
    let subscriber = FileSubscriber {
        sender: Mutex::new(sender.clone()),
        terminal,
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
    let _ = OutputWriter::system()
        .stderr(format!("claude-session: warning: logging unavailable: {error}\n").as_bytes());
}

fn terminal_level(verbosity: u8, environment: &[(OsString, OsString)]) -> Option<Level> {
    let rust_log = environment
        .iter()
        .find(|(key, _)| key == "RUST_LOG")
        .and_then(|(_, value)| value.to_str());
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
    match verbosity {
        0 => None,
        1 => Some(Level::INFO),
        2 => Some(Level::DEBUG),
        _ => Some(Level::TRACE),
    }
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
