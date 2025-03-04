use std::path::PathBuf;

use clap::Parser;

pub mod movement;
pub mod types;

#[derive(Parser)]
pub struct Args {
    pub input: PathBuf,
}

fn main() {
    println!("Hello, world!");
}
