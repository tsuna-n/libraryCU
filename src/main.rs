use std::process::ExitCode;

fn main() -> ExitCode {
    match librarycu::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lcu: {error:#}");
            ExitCode::FAILURE
        }
    }
}
