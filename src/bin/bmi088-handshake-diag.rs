use eadai::bmi088_diag;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), eadai::error::AppError> {
    let config = bmi088_diag::parse_args(std::env::args().skip(1))?;
    bmi088_diag::run(config)
}
