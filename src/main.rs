pub mod io;
pub mod config;
pub mod shape;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::Parser;
use config::FabConfig;
use log::error;

use crate::{io::svg_input::process_svg, io::svg_output::make_svg};


#[derive(Parser)]
pub struct Args {
    /// Path to the fabrication config.
    /// The config is read from stdin if no path is provided.
    pub config: Option<PathBuf>,

    /// Output directory.
    pub output: PathBuf,
}


fn main() {
    if let Err(_) = std::env::var("RUST_LOG") {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }

    env_logger::init();
    let args = Args::parse();

    let config = match get_config(&args) {
        Ok(config) => config,
        Err(err) => {
            error!("{err}");
            std::process::exit(1);
        },
    };

    if let Err(err) = run(args.output, config) {
        error!("{err}");
        std::process::exit(1);
    }
}


fn get_config(args: &Args) -> Result<FabConfig> {
    let config = if let Some(config) = &args.config {
        serde_norway::from_reader(std::fs::File::open(config)?)?
    } else {
        serde_norway::from_reader(std::io::stdin())?
    };
    Ok(config)
}


fn run(outdir: PathBuf, config: FabConfig) -> Result<()> {
    if !outdir.exists() {
        std::fs::create_dir_all(&outdir)?;
    }
    ensure!(outdir.is_dir(), "{:?} should be a directory", outdir);

    let name = config.name;

    for (i, job) in config.jobs.into_iter().enumerate() {
        let mut content = String::new();
        let parser = svg::open(&job.input, &mut content)?;
        let primitives = process_svg(parser)?;

        let mut data = primitives.into_fab_data(&config.shared);
        if let Some(distance) = job.offset() {
            data.offset(distance, config.shared.resolution);
        }

        let output_path = outdir.join(format!("{name}-{i:02}.svg"));

        let document = make_svg(data);
        svg::save(output_path, &document)?;
    }

    Ok(())
}
