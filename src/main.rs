use std::path::PathBuf;

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use contour::ContourBuilder;
use nalgebra::point;
use svg::{node::element::{path::{Command, Data, Position}, tag, Group, Path}, parser::Event, Document};
use types::Float;

pub mod contour;
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
    use contour::ContourFinalisation as CF;

    let mut contours = vec![];
    let mut thickness = None;
    let mut view_box = None;

    let mut content = String::new();
    for event in svg::open(&args.input, &mut content)? {
        match event {
            Event::Tag(tag::SVG, _, attrs) => {
                if let Some(vb) = attrs.get("viewBox") {
                    view_box = Some(vb.to_string());
                }
            },
            Event::Instruction(..) => {},
            Event::Declaration(..) => {},
            Event::Text(..) => {},
            Event::Tag(tag::Description, ..)
            | Event::Tag(tag::Text, ..)
            | Event::Tag(tag::Title, ..) => {},
            Event::Tag(tag::Group, _, attrs) => {
                let allowed = |(k, v): (&str, &str)| {
                    k == "style" || (k == "transform" && v == "translate(0 0) scale(1 1)")
                };

                if let Some(style) = attrs.get("style") {
                    for css in style.split(";") {
                        let css = css.trim();
                        if let Some((key, value)) = css.split_once(":") {
                            if key == "stroke-width" {
                                thickness = Some(value.parse()?);
                            }
                        }
                    }
                }

                if attrs.iter().any(|(k, v)| !allowed((k, v))) {
                    bail!("Group has a banned attribute: {attrs:?}");
                }
            },
            Event::Tag(tag::Path, _, attrs) => {
                let data = attrs.get("d").unwrap();
                let data = Data::parse(data).unwrap();

                let mut contour = ContourBuilder::new_empty();

                for command in data.iter() {
                    match command {
                        &Command::Move(Position::Absolute, ref params) => {
                            ensure!(params.len() % 2 == 0);
                            for p in params.chunks(2) {
                                if let [x, y] = p {
                                    contour.do_move(point![*x, *y])?;
                                }
                            }
                        },
                        &Command::Line(Position::Absolute, ref params) => {
                            ensure!(params.len() % 2 == 0);
                            for p in params.chunks(2) {
                                if let [x, y] = p {
                                    contour.do_line(point![*x, *y])?;
                                }
                            }
                        },
                        &Command::Close => {
                            contour.do_close()?;
                        },
                        command => {
                            eprintln!("Unsupported path command {command:?}");
                        },
                    }
                }

                let contour = match contour.build()? {
                    CF::Contour(contour) => Ok(contour),
                    CF::Unclosed(contour) => {
                        contour.inflate(thickness.context("Tried to inflate, but no thickness was set")?)
                    },
                };

                match contour {
                    Ok(contour) => {
                        contours.push(contour);
                    },
                    Err(err) => {
                        eprintln!("Failed to build a contour: {err}");
                    },
                }
            },
            Event::Tag(tag::Circle, _, attrs) => {
                let cx: Float = attrs.get("cx").context("No 'cx' on circle")?.parse()?;
                let cy: Float = attrs.get("cy").context("No 'cy' on circle")?.parse()?;
                let r: Float = attrs.get("r").context("No 'r' on circle")?.parse()?;

                let sides = if r < 0.5 {
                    24
                } else if r < 2.0 {
                    48
                } else {
                    72
                };

                contours.push(
                    ContourBuilder::new_circle(point![cx, cy], r, sides),
                );
            },
            event => {
                eprintln!("Unsupported event {event:?}");
            }
        }
    }

    let mut group = Group::new()
        .set("fill", "black")
        .set("stroke", "none");

    for contour in &contours {
        let first = contour.points().next().unwrap();

        let mut data = Data::new()
            .move_to((first.x, first.y));

        for p in contour.points().skip(1) {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        let path = Path::new()
            .set("d", data);

        group = group.add(path);
    }

    let document = Document::new()
        .add(group)
        .set("viewBox", view_box.context("viewBox was not encountered")?);

    svg::save(args.output()?, &document)?;

    Ok(())
}


#[cfg(test)]
mod tests {
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
    fn unclosed_wont_crash() {
        let args = make_args("unclosed.svg");
        run(args).unwrap();
    }
}
