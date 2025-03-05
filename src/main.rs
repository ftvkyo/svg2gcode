use std::path::PathBuf;

use anyhow::{ensure, Context, Result};
use clap::Parser;

pub mod contour;
pub mod process;
pub mod types;

#[derive(Parser)]
pub struct Args {
    pub input: PathBuf,

    #[clap(default_value = "tmp/")]
    pub outdir: PathBuf,
}

impl Args {
    pub fn output(&self) -> Result<PathBuf> {
        if !self.outdir.exists() {
            std::fs::create_dir_all(&self.outdir)?;
        }
        ensure!(self.outdir.is_dir(), "{:?} should be a directory", self.outdir);

        let file_name = self.input.file_name().context("Input path does not contain a file name")?;
        Ok(self.outdir.join(file_name))
    }
}


fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("ERROR: {err}");
    }
}


fn run(args: Args) -> Result<()> {
    let document = process::process(&args.input)?;
    svg::save(args.output()?, &document)?;
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
            outdir: PathBuf::from(OUTDIR),
        }
    }

    #[test]
    fn nested_groups() {
        let args = make_args("nested-groups.svg");
        run(args).unwrap();
    }

    #[test]
    fn unclosed() {
        let args = make_args("unclosed.svg");
        run(args).unwrap();
    }
}
