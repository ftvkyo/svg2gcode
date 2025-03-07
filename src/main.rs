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

    /// Resolution to use for line caps and joints
    #[clap(long)]
    pub resolution_lines: Option<geo::Float>,

    /// Resolution to use for circles
    #[clap(long)]
    pub resolution_circles: Option<geo::Float>,
}


fn main() {
    env_logger::init();
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
mod tests {
    use log::info;

    use crate::geo::debug::init_test_logger;

    use super::*;

    const INDIR: &'_ str = "test-data/";
    const OUTDIR: &'_ str = "tmp/test-output/";

    fn run_file_tests(input: &str) -> Result<()> {
        init_test_logger();

        let input = PathBuf::from(INDIR).join(input);
        let input_stem = input.file_stem().expect("input filename stem").to_str().expect("input filename UTF-8");
        let input_extension = input.extension().expect("input filename extension").to_str().expect("input filename UTF-8");
        let outdir = PathBuf::from(OUTDIR);

        let output_default = outdir.join(format!("{input_stem}.{input_extension}"));

        info!("Processing {output_default:?}");

        let res1 = run(Args {
            input: input.clone(),
            output: output_default,
            offset: 0.0,
            resolution_lines: None,
            resolution_circles: None,
        });

        let output_lowres = format!("{input_stem}-lowres.{input_extension}");
        let output_lowres = outdir.join(output_lowres);

        info!("Processing {output_lowres:?}");

        let res2 = run(Args {
            input: input.clone(),
            output: output_lowres,
            offset: 0.0,
            resolution_lines: Some(5.0),
            resolution_circles: Some(5.0),
        });

        let output_offset = format!("{input_stem}-offset.{input_extension}");
        let output_offset = outdir.join(output_offset);

        info!("Processing {output_offset:?}");

        let res3 = run(Args {
            input: input.clone(),
            output: output_offset,
            offset: 5.0,
            resolution_lines: None,
            resolution_circles: None,
        });

        let output_offset_lowres = format!("{input_stem}-offset-lowres.{input_extension}");
        let output_offset_lowres = outdir.join(output_offset_lowres);

        info!("Processing {output_offset_lowres:?}");

        let res4 = run(Args {
            input: input.clone(),
            output: output_offset_lowres,
            offset: 5.0,
            resolution_lines: Some(5.0),
            resolution_circles: Some(5.0),
        });

        res1.and(res2).and(res3).and(res4)
    }

    #[test]
    fn file_00_shapes() -> Result<()> {
        run_file_tests("00-shapes.svg")
    }

    #[test]
    fn file_01_nested_groups() -> Result<()> {
        run_file_tests("01-nested-groups.svg")
    }

    #[test]
    fn file_02_line_merging() -> Result<()> {
        run_file_tests("02-line-merging.svg")
    }

    #[test]
    fn file_03_contour_merging() -> Result<()> {
        run_file_tests("03-contour-merging.svg")
    }
}
