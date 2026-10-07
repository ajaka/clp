use std::{env, process};

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    if let Err(e) = clp::begin(args) {
        eprintln!("{}", e);
        process::exit(1);
    }
}
