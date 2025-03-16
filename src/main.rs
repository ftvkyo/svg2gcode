pub mod io;
pub mod config;
pub mod fab;
pub mod shape;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::Parser;
use config::FabConfig;
use fab::FabData;
use io::gcode::make_gcode;
use log::{error, info};

use crate::{io::svg_input::process_svg, io::svg_output::make_svg};


#[derive(Parser)]
pub struct Args {
    /// Path to the fabrication config.
    pub config: PathBuf,
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

    if let Err(err) = run(config) {
        error!("{err}");
        std::process::exit(1);
    }
}


fn get_config(args: &Args) -> Result<FabConfig> {
    Ok(serde_norway::from_reader(std::fs::File::open(&args.config)?)?)
}


fn run(config: FabConfig) -> Result<()> {
    if !config.outdir.exists() {
        std::fs::create_dir_all(&config.outdir)?;
    }
    ensure!(config.outdir.is_dir(), "{:?} should be a directory", config.outdir);

    let name = config.name;

    let mut fds: Vec<FabData> = Vec::with_capacity(config.jobs.len());

    for (i, job) in config.jobs.into_iter().enumerate() {
        let mut content = String::new();
        let parser = svg::open(&job.input, &mut content)?;
        let primitives = process_svg(parser)?;

        info!("Processed the SVG for job {i:02}");

        let fd = FabData::new(&config.shared, job, primitives)?;

        info!("Generated fabdata for job {i:02}");

        let output_path = config.outdir.join(format!("{name}-{i:02}.ngc"));
        let ngc = make_gcode(&config.shared, &fd);
        std::fs::write(output_path, ngc)?;

        info!("Produced G-Code for job {i:02}");

        fds.push(fd);
    }

    let document = make_svg(&fds);
    let output_path = config.outdir.join(format!("{name}.svg"));
    svg::save(output_path, &document)?;

    Ok(())
}
