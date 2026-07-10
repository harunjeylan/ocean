mod ocean_cli;

fn main() {
    match ocean_cli::run() {
        Ok(()) => {}
        Err(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
    }
}
