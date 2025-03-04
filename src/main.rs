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

    let mut content = String::new();
    for event in svg::open(args.input, &mut content)? {
        match event {
            Event::Instruction(..) => {},
            Event::Declaration(..) => {},
            Event::Text(..) => {},
            Event::Tag(tag::SVG, ..)
            | Event::Tag(tag::Description, ..)
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
                    CF::Deflated(contour) => {
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

    let mut group = Group::new();

    let mut min_x: Float = 0.0;
    let mut max_x: Float = 0.0;
    let mut min_y: Float = 0.0;
    let mut max_y: Float = 0.0;

    for contour in &contours {
        for p in contour.points() {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
    }

    for contour in &contours {
        let first = contour.points().next().unwrap();

        let mut data = Data::new()
            .move_to((first.x, first.y));

        for p in contour.points().skip(1) {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        let path = Path::new()
            .set("fill", "black")
            .set("stroke", "none")
            .set("d", data);

        group = group.add(path);
    }

    let document = Document::new()
        .set("viewBox", (min_x - 5.0, min_y - 5.0, max_x - min_x + 10.0, max_y - min_y + 10.0))
        .add(group);

    println!("{}", document.to_string());

    Ok(())
}
