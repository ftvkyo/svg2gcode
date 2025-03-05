use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use nalgebra::point;
use svg::{node::element::{path::{Command, Data, Position}, tag, Group, Path}, parser::Event, Document};

use crate::types::Float;
use crate::contour::{ContourFinalisation as CF, ContourBuilder};


struct Attributes {
    stroke_width: Vec<Option<Float>>,
}

impl Attributes {
    pub fn new() -> Self {
        Self {
            stroke_width: vec![],
        }
    }

    pub fn group_push(&mut self, attrs: &HashMap<String, svg::node::Value>) -> Result<()> {
        let mut stroke_width: Option<Float> = None;

        for (attr, val) in attrs {
            match attr.as_str() {
                "style" => {
                    for prop in val.split(";") {
                        if let Some((prop_key, prop_val)) = prop.split_once(":") {
                            let prop_key = prop_key.trim();
                            let prop_val = prop_val.trim();

                            match prop_key {
                                "stroke-width" => {
                                    stroke_width = Some(prop_val.parse()?);
                                },
                                _ => {}
                            }
                        }
                    }
                },
                "transform" => {
                    ensure!(*val == "translate(0 0) scale(1 1)", "only no-op transform is supported");
                },
                attr => {
                    bail!("Attribute {attr} (value {val}) is not supported");
                },
            }
        }

        self.stroke_width.push(stroke_width);

        Ok(())
    }

    pub fn group_pop(&mut self) -> Result<()> {
        ensure!(self.stroke_width.pop().is_some());
        Ok(())
    }

    pub fn get_stroke_width(&self) -> Result<Float> {
        let val = self.stroke_width.iter().filter_map(|x| *x).last();
        if let Some(val) = val {
            return Ok(val);
        }

        bail!("default (not set) stroke width is not supported");
    }
}


pub fn process(file: impl AsRef<std::path::Path>) -> Result<svg::Document> {
    let mut style = Attributes::new();

    let mut contours = vec![];
    let mut view_box = None;

    let mut content = String::new();
    for event in svg::open(file.as_ref(), &mut content)? {
        match event {

            /* Ignore some events */

            | Event::Instruction(..)
            | Event::Declaration(..)
            | Event::Text(..)
            | Event::Tag(tag::Description, ..)
            | Event::Tag(tag::Text, ..)
            | Event::Tag(tag::Title, ..) => {},

            /* Get the view box */

            Event::Tag(tag::SVG, _, attrs) => {
                if let Some(vb) = attrs.get("viewBox") {
                    view_box = Some(vb.to_string());
                }
            },

            /* Handle group opening and closing */

            Event::Tag(tag::Group, tag::Type::Start, attrs) => {
                style.group_push(&attrs)?;
            },
            Event::Tag(tag::Group, tag::Type::End, ..) => {
                style.group_pop()?;
            }

            /* Handle paths */

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
                        let stroke_width = style.get_stroke_width()?;
                        contour.expand(stroke_width)
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

            /* Handle circles */

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

            /* Everything else is not supported */

            event => {
                eprintln!("Unsupported event {event:?}");
            }
        }
    }

    let mut g_contours = Group::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1);

    for contour in &contours {
        let first = contour.points().next().unwrap();

        let mut data = Data::new()
            .move_to((first.x, first.y));

        for p in contour.points().skip(1) {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        let path = Path::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke");

        g_contours = g_contours.add(path);
    }

    let document = Document::new()
        .add(g_contours)
        .set("viewBox", view_box.context("viewBox was not encountered")?);

    Ok(document)
}
