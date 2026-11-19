use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};

static QUIET: AtomicBool = AtomicBool::new(false);

pub fn configure(quiet: bool, no_color: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
    let no_color = no_color || std::env::var_os("NO_COLOR").is_some();
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
