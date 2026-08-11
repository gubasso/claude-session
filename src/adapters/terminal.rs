//! Controlling-terminal availability boundary.

use std::{fs::OpenOptions, io};

pub(crate) trait Terminal {
    fn available(&self) -> io::Result<bool>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemTerminal;

impl Terminal for SystemTerminal {
    fn available(&self) -> io::Result<bool> {
        match OpenOptions::new().read(true).write(true).open("/dev/tty") {
            Ok(_) => Ok(true),
            Err(error) if matches!(error.raw_os_error(), Some(6 | 25)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
