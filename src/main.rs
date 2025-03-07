use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::Parser;

pub mod geo;
pub mod process;


#[derive(Parser)]
pub struct Args {
    pub input: PathBuf,
    pub output: PathBuf,

    /// Grow every contour by this much
    #[clap(long, default_value = "0")]
    pub offset: geo::Float,

    /// How many segments to use for line caps
    #[clap(long, default_value = "12")]
    pub segments_caps: usize,

    /// How many segments to use for circles
    #[clap(long, default_value = "24")]
    pub segments_circles: usize,
}


fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("ERROR: {err}");
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

    let document = process::process(&args)?;
    svg::save(&args.output, &document)?;
    Ok(())
}


#[cfg(test)]
mod test_files {
    use super::*;

    const INDIR: &'_ str = "test-data/";
    const OUTDIR: &'_ str = "tmp/test-output/";

    fn make_args(input: &str) -> Args {
        Args {
            input: PathBuf::from(INDIR).join(input),
            output: PathBuf::from(OUTDIR).join(input),
            offset: 0.0,
            segments_caps: 12,
            segments_circles: 24,
        }
    }

    #[test]
    fn mix() -> Result<()> {
        let args = make_args("mix.svg");
        run(args)
    }

    #[test]
    fn mix_with_offset() -> Result<()> {
        let mut args = make_args("mix.svg");
        args.output.set_file_name("mix-with-offset.svg");
        args.offset = 1.5;
        run(args)
    }

    #[test]
    fn nested_groups() -> Result<()> {
        let args = make_args("nested-groups.svg");
        run(args)
    }

    #[test]
    fn separate_lines() -> Result<()> {
        let args = make_args("separate-lines.svg");
        run(args)
    }

    #[test]
    fn unclosed() -> Result<()> {
        let args = make_args("unclosed.svg");
        run(args)
    }
}
