use std::process::ExitCode;

fn main() -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("No working directory: {error}");
            return ExitCode::FAILURE;
        }
    };
    let code = kempelen_tag::run(
        &root,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    );
    ExitCode::from(code)
}
