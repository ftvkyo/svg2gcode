use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use nalgebra::point;
use svg::{node::element::{path::{Command, Data, Position}, tag, Group}, parser::Event, Document};

use crate::{contour::Contour, types::Float};
use crate::contour::{ContourFinalisation as CF, ContourBuilder};


struct SvgContext {
    view_box: Option<String>,
    stroke_width: Vec<Option<Float>>,
}

impl SvgContext {
    pub fn new() -> Self {
        Self {
            view_box: None,
            stroke_width: vec![],
        }
    }

    pub fn svg_push(&mut self, attrs: &HashMap<String, svg::node::Value>) -> Result<()> {
        for (attr, val) in attrs {
            match attr.as_str() {
                "viewBox" => {
                    ensure!(self.view_box.replace(val.to_string()).is_none());
                },
                _ => {}
            }
        }

        Ok(())
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

    pub fn get_view_box(&self) -> Result<&str> {
        self.view_box.as_ref().map(|s| s.as_str()).context("No view box?")
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
    let mut ctx = SvgContext::new();

    let mut contours = vec![];

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

            /* Handle svg opening and closing */

            Event::Tag(tag::SVG, tag::Type::Start, attrs) => {
                ctx.svg_push(&attrs)?;
            },
            Event::Tag(tag::SVG, tag::Type::End, ..) => {},

            /* Handle group opening and closing */

            Event::Tag(tag::Group, tag::Type::Start, attrs) => {
                ctx.group_push(&attrs)?;
            },
            Event::Tag(tag::Group, tag::Type::End, ..) => {
                ctx.group_pop()?;
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
                        let stroke_width = ctx.get_stroke_width()?;
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

    make_svg(ctx.get_view_box()?, contours)
}


fn make_svg(view_box: &str, contours: Vec<Contour>) -> Result<svg::Document> {
    let mut g_contours = Group::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1);

    for contour in &contours {
        let path: svg::node::element::Path = contour.into();
        g_contours = g_contours.add(path);
    }

    let document = Document::new()
        .add(g_contours)
        .set("viewBox", view_box);

    Ok(document)
}
