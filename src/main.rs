
pub mod input;
pub mod output;
pub mod shape;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::Parser;
use log::error;

use crate::{input::process_svg, output::make_svg};

#[derive(Parser)]
pub struct Args {
    pub input: PathBuf,
    pub output: PathBuf,
}


fn main() {
    env_logger::init();
    let args = Args::parse();
    if let Err(err) = run(args) {
        error!("{err}");
    }
}


fn run(args: Args) -> Result<()> {
    let outdir = args.output.parent();
    if let Some(outdir) = outdir {
        if !outdir.exists() {
            std::fs::create_dir_all(&outdir)?;
        }
        ensure!(outdir.is_dir(), "{outdir:?} should be a directory");
    }

    let mut content = String::new();
    let parser = svg::open(&args.input, &mut content)?;
    let shapes = process_svg(parser)?;

    let document = make_svg(shapes);
    svg::save(&args.output, &document)?;

    Ok(())
}
