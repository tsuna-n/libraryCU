use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};

static QUIET: AtomicBool = AtomicBool::new(false);
static NO_COLOR: AtomicBool = AtomicBool::new(false);

pub fn configure(quiet: bool, no_color: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
    let no_color = no_color || std::env::var_os("NO_COLOR").is_some();
    NO_COLOR.store(no_color, Ordering::Relaxed);
    if no_color || !io::stdout().is_terminal() {
        colored::control::set_override(false);
    }
}

pub fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

pub fn warning(message: &str) {
    if !is_quiet() {
        eprintln!(
            "lbc: warning: {}",
            crate::security::redact_sensitive(message)
        );
    }
}

/// Return whether progress was displayed so callers only clear visible text.
pub fn progress(message: &str) -> bool {
    if is_quiet() || !io::stderr().is_terminal() {
        return false;
    }
    eprint!("{message}");
    let _ = io::stderr().flush();
    true
}

pub fn clear_progress() {
    if NO_COLOR.load(Ordering::Relaxed) {
        eprintln!();
    } else {
        eprint!("\r\x1b[2K");
    }
    let _ = io::stderr().flush();
}
