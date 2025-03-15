
pub mod input;
pub mod output;
pub mod shape;
pub mod transform;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::Parser;
use log::error;
use transform::polygons_unite;

use crate::{input::process_svg, output::make_svg};

#[derive(Parser)]
pub struct Args {
    pub input: PathBuf,
    pub output: PathBuf,

    #[clap(long, default_value = "0.0")]
    pub offset: f64,

    #[clap(long, default_value = "0.1")]
    pub resolution: f64,
}


fn main() {
    if let Err(_) = std::env::var("RUST_LOG") {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }

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
    let polygons = polygons_unite(shapes.polygons(args.offset, args.resolution));
    let document = make_svg(polygons);
    svg::save(&args.output, &document)?;

    Ok(())
}
