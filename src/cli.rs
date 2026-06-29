fn main() {
    match ocean::ocean_cli::run() {
        Ok(()) => {}
        Err(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
    }
}
