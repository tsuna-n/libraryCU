use std::process::ExitCode;

#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() -> ExitCode {
    reset_sigpipe();
    match librarycube::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lbc: {error:#}");
            ExitCode::FAILURE
        }
    }
}
