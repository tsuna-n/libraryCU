use std::process::ExitCode;

fn main() -> ExitCode {
    match librarycube::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lbc: {error:#}");
            ExitCode::FAILURE
        }
    }
}
