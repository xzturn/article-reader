use std::process::ExitCode;

fn main() -> ExitCode {
    match article_reader::cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("❌ {e}");
            ExitCode::FAILURE
        }
    }
}
