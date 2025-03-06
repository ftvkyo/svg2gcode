use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::Parser;

pub mod geo;
pub mod process;

#[derive(Parser)]
pub struct Args {
    pub input: PathBuf,
    pub output: PathBuf,

    #[clap(default_value = "0")]
    pub offset: geo::Float,
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

    let document = process::process(&args.input, args.offset)?;
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
        }
    }

    #[test]
    fn mix() {
        let args = make_args("mix.svg");
        run(args).unwrap();
    }

    #[test]
    fn mix_with_offset() {
        let mut args = make_args("mix.svg");
        args.output.set_file_name("mix-with-offset.svg");
        args.offset = 1.5;
        run(args).unwrap();
    }

    #[test]
    fn nested_groups() {
        let args = make_args("nested-groups.svg");
        run(args).unwrap();
    }

    #[test]
    fn separate_lines() {
        let args = make_args("separate-lines.svg");
        run(args).unwrap();
    }

    #[test]
    fn unclosed() {
        let args = make_args("unclosed.svg");
        run(args).unwrap();
    }
}
