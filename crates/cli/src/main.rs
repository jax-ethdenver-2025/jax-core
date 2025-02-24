#![allow(dead_code)]
#![allow(clippy::result_large_err)]

mod args;
mod ops;
mod state;
mod version;

use args::{Args, Op, Parser};
use state::AppState;

#[tokio::main]
async fn main() {
    // Run the app and capture any errors
    let args = Args::parse();
    let state = match AppState::try_from(&args) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("could not setup app state: {}", e);
            std::process::exit(1);
        }
    };

    let op = args.command.clone();
    match op.execute(&state).await {
        Ok(r) => {
            println!("{}", r);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {:?}", e);
            std::process::exit(1);
        }
    };
}
