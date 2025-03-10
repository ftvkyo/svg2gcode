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
    ensure!(args.offset >= 0.0);

    if let Some(resolution) = args.resolution_lines {
        ensure!(resolution > 0.0);
    }

    if let Some(resolution) = args.resolution_circles {
        ensure!(resolution > 0.0);
    }

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
    use crate::geo::{debug::init_test_logger, Float};

    use super::*;

    const INDIR: &'_ str = "test-data/";
    const OUTDIR: &'_ str = "tmp/test-output/";

    fn test_file(input: &str, offset: Option<Float>, resolution: Option<Float>) -> Result<()> {
        init_test_logger();

        let input = PathBuf::from(INDIR).join(input);
        let input_stem = input.file_stem().expect("input filename stem").to_str().expect("input filename UTF-8");
        let input_extension = input.extension().expect("input filename extension").to_str().expect("input filename UTF-8");
        let outdir = PathBuf::from(OUTDIR);

        let suffix = match (offset, resolution) {
            (None, None) => "",
            (None, Some(_)) => "-lowres",
            (Some(_), None) => "-offset",
            (Some(_), Some(_)) => "-offset-lowres",
        };

        let output = outdir.join(format!("{input_stem}{suffix}.{input_extension}"));

        run(Args {
            input: input.clone(),
            output,
            offset: offset.unwrap_or(0.0),
            resolution_lines: resolution,
            resolution_circles: resolution,
        })
    }

    #[test]
    fn file_00_shapes() -> Result<()> {
        test_file("00-shapes.svg", None, None)
    }

    #[test]
    fn file_00_shapes_lowres() -> Result<()> {
        test_file("00-shapes.svg", None, Some(5.0))
    }

    #[test]
    fn file_00_shapes_offset() -> Result<()> {
        test_file("00-shapes.svg", Some(5.0), None)
    }

    #[test]
    fn file_00_shapes_offset_lowres() -> Result<()> {
        test_file("00-shapes.svg", Some(5.0), Some(5.0))
    }

    #[test]
    fn file_01_nested_groups() -> Result<()> {
        test_file("01-nested-groups.svg", None, None)
    }

    #[test]
    fn file_01_nested_groups_lowres() -> Result<()> {
        test_file("01-nested-groups.svg", None, Some(5.0))
    }

    #[test]
    fn file_01_nested_groups_offset() -> Result<()> {
        test_file("01-nested-groups.svg", Some(5.0), None)
    }

    #[test]
    fn file_01_nested_groups_offset_lowres() -> Result<()> {
        test_file("01-nested-groups.svg", Some(5.0), Some(5.0))
    }

    #[test]
    fn file_02_line_merging() -> Result<()> {
        test_file("02-line-merging.svg", None, None)
    }

    #[test]
    fn file_02_line_merging_lowres() -> Result<()> {
        test_file("02-line-merging.svg", None, Some(5.0))
    }

    #[test]
    fn file_02_line_merging_offset() -> Result<()> {
        test_file("02-line-merging.svg", Some(5.0), None)
    }

    #[test]
    fn file_02_line_merging_offset_lowres() -> Result<()> {
        test_file("02-line-merging.svg", Some(5.0), Some(5.0))
    }

    #[test]
    fn file_03_contour_merging() -> Result<()> {
        test_file("03-contour-merging.svg", None, None)
    }

    #[test]
    fn file_03_contour_merging_lowres() -> Result<()> {
        test_file("03-contour-merging.svg", None, Some(5.0))
    }

    #[test]
    fn file_03_contour_merging_offset() -> Result<()> {
        test_file("03-contour-merging.svg", Some(5.0), None)
    }

    #[test]
    fn file_03_contour_merging_offset_lowres() -> Result<()> {
        test_file("03-contour-merging.svg", Some(5.0), Some(5.0))
    }
}
