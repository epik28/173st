use std::process::ExitCode;

fn main() -> ExitCode {
    match beta_viewer::loader::run_args(std::env::args_os().skip(1)) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("view_probe: {error}");
            ExitCode::FAILURE
        }
    }
}
