use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use nalgebra::point;
use svg::{node::element::{path::{Command, Data, Position}, tag, Group}, parser::Event, Document};

use crate::geo::{contour, shape, Float};


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


pub fn process(file: impl AsRef<std::path::Path>, offset: Float) -> Result<svg::Document> {
    let mut ctx = SvgContext::new();

    let mut g_originals = Group::new()
        .set("opacity", "50%");
    let mut contours: Vec<contour::Contour> = vec![];

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

            Event::Tag(tag::Path, tag::Type::Empty, attrs) => {
                let data = attrs.get("d").context("No 'd' attribute on a path?")?;

                let mut builder = shape::PathBuilder::new();

                // Mwahahaha
                'out: loop {
                    for command in Data::parse(data)?.iter() {
                        match command {
                            &Command::Move(Position::Absolute, ref params) => {
                                ensure!(params.len() % 2 == 0);
                                for p in params.chunks(2) {
                                    if let [x, y] = p {
                                        builder.add_moveto(point![*x, *y])?;
                                    }
                                }
                            },
                            &Command::Line(Position::Absolute, ref params) => {
                                ensure!(params.len() % 2 == 0);
                                for p in params.chunks(2) {
                                    if let [x, y] = p {
                                        builder.add_lineto(point![*x, *y])?;
                                    }
                                }
                            },
                            &Command::Close => {
                                let contour = builder.into_contour()?;
                                contours.push(contour);

                                break 'out;
                            },
                            command => {
                                eprintln!("Unsupported path command {command:?}");
                            },
                        }
                    }

                    let line = builder.into_line(ctx.get_stroke_width()?)?;
                    contours.push(line.into_contour(12)?);

                    break 'out;
                }

                // Save the original shape too

                let mut original = svg::node::element::Path::new();
                original = fix_attributes(original, attrs.clone())?;
                original = fix_stroke_width(original, || ctx.get_stroke_width())?;
                g_originals = g_originals.add(original);
            },

            /* Handle circles */

            Event::Tag(tag::Circle, tag::Type::Empty, attrs) => {
                let cx: Float = attrs.get("cx").context("No 'cx' on circle")?.parse()?;
                let cy: Float = attrs.get("cy").context("No 'cy' on circle")?.parse()?;
                let r: Float = attrs.get("r").context("No 'r' on circle")?.parse()?;

                contours.push(shape::Circle::new(point![cx, cy], r).into_contour(24)?);

                // Save the original shape too

                let mut original = svg::node::element::Circle::new();
                original = fix_attributes(original, attrs.clone())?;
                g_originals = g_originals.add(original);
            },

            /* Everything else is not supported */

            event => {
                eprintln!("Unsupported event {event:?}");
            }
        }
    }

    for contour in &mut contours {
        contour.grow(offset)?;
    }

    make_svg(ctx.get_view_box()?, contours, g_originals)
}


fn fix_attributes<T: svg::Node>(mut node: T, original_attrs: svg::node::Attributes) -> Result<T> {
    let attrs = node.get_attributes_mut().context("No attributes?")?;
    *attrs = original_attrs;

    let mut make_gray = |attr| {
        if let Some(attr) = attrs.get_mut(attr) {
            if *attr != "none" {
                *attr = "gray".into();
            }
        }
    };

    make_gray("fill");
    make_gray("stroke");

    Ok(node)
}


fn fix_stroke_width<T: svg::Node>(mut node: T, get_stroke_width: impl Fn() -> Result<Float>) -> Result<T> {
    let attrs = node.get_attributes_mut().context("No attributes?")?;

    if !attrs.contains_key("stroke-width") {
        attrs.insert("stroke-width".to_string(), get_stroke_width()?.into());
    }

    Ok(node)
}


fn make_svg(view_box: &str, contours: Vec<contour::Contour>, g_originals: Group) -> Result<svg::Document> {
    let mut g_contours = Group::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1);

    for contour in contours {
        g_contours = g_contours.add(contour.svg()?);
    }

    let document = Document::new()
        .add(g_originals)
        .add(g_contours)
        .set("viewBox", view_box);

    Ok(document)
}
