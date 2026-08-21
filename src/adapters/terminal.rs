//! Controlling-terminal boundary: availability, consent, secret entry, naming.
//!
//! The first three address `/dev/tty` rather than standard input, which is what
//! makes a confirmation survive `something | claude-session-rs account remove
//! work` and makes a pasted token unreadable from a redirected stream. Nothing
//! consults `isatty(0)`; opening `/dev/tty` read-write is the predicate.
//!
//! Naming is the exception, and deliberately so: `/dev/tty` is the alias every
//! process shares, so it answers whether a terminal is there and never which
//! one. That question is `adapters::host::controlling_terminal`'s.

use std::{
    fs::{File, OpenOptions},
    io::{self, Read as _, Write as _},
};

use crate::domain::{
    namespace::Kind,
    secret::{Secret, SecretError},
    terminal::Terminal as TerminalIdentity,
};

pub(crate) trait Terminal {
    /// Reports whether a controlling terminal can be opened.
    fn available(&self) -> io::Result<bool>;
    /// Writes a prompt and reads one echoed line.
    ///
    /// `Ok(None)` is end of input, which the consent rule reads as declining.
    fn ask(&self, prompt: &str) -> io::Result<Option<String>>;
    /// Writes a prompt and reads one line with terminal echo disabled.
    fn read_secret(&self, prompt: &str) -> io::Result<Result<Secret, SecretError>>;
    /// Names the terminal this run's child state directory belongs to.
    ///
    /// `Ok(None)` is the refusal rung: neither a controlling terminal nor a
    /// session leader named anything usable, and the caller must fail rather
    /// than invent a directory ([ADR-0102]).
    ///
    /// A rung whose namespace cannot be read names nothing and the ladder falls
    /// past it, so an unreadable `/proc` costs a rung rather than adding a
    /// failure of its own ([ADR-0107]).
    ///
    /// [ADR-0102]: ../../docs/decisions/ADR-0102-key-child-state-by-terminal.md
    /// [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md
    fn identity(&self) -> io::Result<Option<TerminalIdentity>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemTerminal;

// The leader's start-time read lives in `adapters::host` as `process_started`,
// shared with the liveness judgment that re-asks the naming question
// ([ADR-0111]).
//
// [ADR-0111]: ../../docs/decisions/ADR-0111-collect-only-the-provably-dead-session.md
use crate::adapters::host::process_started as leader_started;

// The namespace read lives in `adapters::host`, shared with the peer-scope
// derivation. `None` when `/proc` does not answer, which makes the rung asking
// unavailable ([ADR-0107]).
//
// [ADR-0107]: ../../docs/decisions/ADR-0107-scope-a-terminal-to-its-namespace.md
use crate::adapters::host::namespace;

// The pane read lives in `adapters::host` beside the other `procfs` identity
// reads, because naming the terminal a run belongs to is the same question in
// a different scope ([ADR-0102]).
//
// [ADR-0102]: ../../docs/decisions/ADR-0102-key-child-state-by-terminal.md
use crate::adapters::host::controlling_terminal;

/// `ENXIO` and `EBADF`, the two ways an absent controlling terminal reports.
const NO_TERMINAL: [i32; 2] = [6, 25];

impl Terminal for SystemTerminal {
    fn available(&self) -> io::Result<bool> {
        match open_tty() {
            Ok(_) => Ok(true),
            Err(error)
                if error
                    .raw_os_error()
                    .is_some_and(|code| NO_TERMINAL.contains(&code)) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    fn ask(&self, prompt: &str) -> io::Result<Option<String>> {
        let tty = open_tty()?;
        write_prompt(&tty, prompt)?;
        let line = read_line(&tty)?;
        Ok(line.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn identity(&self) -> io::Result<Option<TerminalIdentity>> {
        // The pane's own pseudo-terminal first, read from this process's
        // controlling-terminal number rather than from a stream or from the
        // alias: `isatty(0)` answers for whatever standard input was pointed
        // at, and `/dev/tty` resolves back to itself, so neither names a pane
        // ([ADR-0062], [ADR-0102]). The mount namespace leads the chain because
        // it carries the devpts instance that issued the device name, so an
        // unreadable one costs this rung and nothing below it.
        if let Some(space) = namespace(Kind::Mount)
            && let Some(device) = controlling_terminal()
            && let Some(terminal) = TerminalIdentity::from_tty(space, &device)
        {
            return Ok(Some(terminal));
        }
        // The process namespace issued both the session id and the process ids
        // `/proc` reports it against, so it is this rung's namespace.
        let Some(space) = namespace(Kind::Pid) else {
            return Ok(None);
        };
        let Ok(sid) = rustix::process::getsid(None) else {
            return Ok(None);
        };
        let raw = sid.as_raw_nonzero().get().unsigned_abs();
        // Without the leader's start time a recycled process id would inherit
        // an earlier session's directory, so a leader whose start time cannot
        // be read is no answer at all.
        let Some(started) = leader_started(raw) else {
            return Ok(None);
        };
        Ok(TerminalIdentity::from_session_leader(space, raw, started))
    }

    fn read_secret(&self, prompt: &str) -> io::Result<Result<Secret, SecretError>> {
        let tty = open_tty()?;
        write_prompt(&tty, prompt)?;
        let (line, trailing) = {
            // The guard restores the saved attributes on every path out,
            // including an unwind. It is the only thing between a bug in the
            // read below and a terminal left with echo off, which is why only
            // the reads live inside its scope.
            let _echo = EchoOff::engage(&tty)?;
            let line = read_line(&tty)?;
            (line, drain(&tty)?)
        };
        // The user's Enter was consumed without being echoed, so the cursor is
        // still on the prompt line and the next writer would overwrite it.
        (&tty).write_all(b"\n")?;
        let line = line.unwrap_or_default();
        if line.contains(&INTERRUPT) {
            // The interrupt key, which `ISIG` would otherwise have turned into
            // a signal that killed this process with echo still off. Answering
            // it here means the terminal is already restored by the time the
            // caller sees the refusal.
            return Ok(Err(SecretError::Cancelled));
        }
        if trailing {
            // Refused rather than truncated, matching standard input: a reader
            // that took the first line would store half of what was pasted and
            // say nothing. The remainder was drained above rather than left
            // queued, so it does not reach the user's shell as typed commands
            // after this process exits.
            return Ok(Err(SecretError::MultipleLines));
        }
        Ok(Secret::parse_line(&line))
    }
}

fn open_tty() -> io::Result<File> {
    OpenOptions::new().read(true).write(true).open("/dev/tty")
}

fn write_prompt(tty: &File, prompt: &str) -> io::Result<()> {
    let mut handle = tty;
    handle.write_all(prompt.as_bytes())?;
    handle.flush()
}

/// Reads one line, returning `None` only when nothing at all arrived.
///
/// Byte-oriented and one byte at a time, because a buffered reader would
/// consume past the newline into whatever the terminal delivers next. Canonical
/// mode stays on, so the kernel has already assembled the line and this loop
/// only finds its end.
fn read_line(tty: &File) -> io::Result<Option<Vec<u8>>> {
    let mut handle = tty;
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        match handle.read(&mut byte)? {
            0 => break,
            _ if byte[0] == b'\n' => {
                line.push(byte[0]);
                break;
            }
            _ => line.push(byte[0]),
        }
    }
    if line.is_empty() {
        Ok(None)
    } else {
        Ok(Some(line))
    }
}

/// Consumes anything the paste queued behind its first newline.
///
/// Returns whether there was any. Asking the driver how much is waiting is what
/// makes this safe: a plain read would block until the user typed something,
/// and the common case is that nothing is waiting at all. Canonical mode counts
/// only completed lines, so a partial second line without its own newline is
/// invisible here and is left for the shell exactly as an interrupted paste
/// would be.
fn drain(tty: &File) -> io::Result<bool> {
    let mut drained = false;
    let mut discard = [0_u8; 256];
    let mut handle = tty;
    while rustix::io::ioctl_fionread(tty)? > 0 {
        let read = handle.read(&mut discard)?;
        if read == 0 {
            break;
        }
        drained = true;
    }
    Ok(drained)
}

/// The interrupt character, delivered as data once `ISIG` is off.
const INTERRUPT: u8 = 0x03;

/// Terminal echo and signal keys, disabled for as long as this value lives.
///
/// `ECHO` is cleared so the credential is not written to the screen. `ISIG` is
/// cleared for a less obvious reason: while it is on, the two keys a user is
/// most likely to press at an unexpected prompt are the two that end this
/// process without unwinding. An interrupt would kill it with echo still off,
/// and a suspend would hand the shell back a terminal that does not echo — and
/// the restore below never runs in either case, because neither is a panic.
///
/// With `ISIG` off, both arrive as ordinary bytes. The interrupt is recognized
/// and answered as a cancellation; every other control byte is refused by the
/// credential's own parser. Either way this value is dropped on the way out and
/// the terminal is restored, which is the property the signals were preventing.
///
/// `ICANON` stays on, so the terminal never enters raw mode and the kernel's
/// line editor keeps working. The cost is that the keys do nothing for the
/// duration of one read; the escape from a prompt nobody wants to answer is
/// Enter or end-of-input, both of which refuse and exit.
struct EchoOff<'a> {
    tty: &'a File,
    saved: rustix::termios::Termios,
}

impl<'a> EchoOff<'a> {
    fn engage(tty: &'a File) -> io::Result<Self> {
        use rustix::termios::{LocalModes, OptionalActions, tcgetattr, tcsetattr};
        let saved = tcgetattr(tty)?;
        let mut quiet = saved.clone();
        quiet.local_modes -= LocalModes::ECHO | LocalModes::ISIG;
        // Flush rather than Now, so type-ahead entered before the prompt is
        // discarded instead of being read as part of the credential.
        tcsetattr(tty, OptionalActions::Flush, &quiet)?;
        Ok(Self { tty, saved })
    }
}

impl Drop for EchoOff<'_> {
    fn drop(&mut self) {
        let _ = rustix::termios::tcsetattr(
            self.tty,
            rustix::termios::OptionalActions::Now,
            &self.saved,
        );
    }
}
