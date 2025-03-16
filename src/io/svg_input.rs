use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use geo::{Coord, LineString, MultiPolygon, Polygon, RemoveRepeatedPoints};
use log::{error, warn};
use svg::{node::element::{path, tag}, parser::Event, Parser};

use crate::{fab::Hole, shape::{Circle, IntoPolygon, ThickLineString}};

pub struct SvgContext {
    stroke_width: Vec<Option<f64>>,
}

impl SvgContext {
    pub fn new() -> Self {
        Self {
            stroke_width: vec![],
        }
    }

    pub fn group_push(&mut self, attrs: &HashMap<String, svg::node::Value>) -> Result<()> {
        let mut stroke_width: Option<f64> = None;

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
        assert!(self.stroke_width.pop().is_some());
        Ok(())
    }

    pub fn get_stroke_width(&self) -> Result<f64> {
        let val = self.stroke_width.iter().filter_map(|x| *x).last();
        if let Some(val) = val {
            return Ok(val);
        }

        bail!("default (not set) stroke width is not supported");
    }
}


#[derive(Clone, Debug)]
pub struct PathBuilder {
    inner: LineString,
}

impl PathBuilder {
    pub fn new() -> Self {
        Self {
            inner: LineString::new(vec![]),
        }
    }

    pub fn get_position(&self) -> Result<Coord> {
        self.inner.0.last().copied().context("Tried to get the current line position, but the line is empty")
    }

    pub fn moveto(mut self, ps: impl Iterator<Item = Coord>) -> Result<Self> {
        ensure!(self.inner.0.len() == 0, "Move To is only supported as the first command");
        self.inner.0.extend(ps);
        Ok(self)
    }

    pub fn lineto(mut self, ps: impl Iterator<Item = Coord>) -> Result<Self> {
        ensure!(self.inner.0.len() > 0, "Line To can not be the first command");
        self.inner.0.extend(ps);
        Ok(self)
    }

    pub fn moveby(self, ps: impl Iterator<Item = Coord>) -> Result<Self> {
        self.moveto(ps)
    }

    pub fn lineby(mut self, ps: impl Iterator<Item = Coord>) -> Result<Self> {
        ensure!(self.inner.0.len() > 0, "Line By can not be the first command");
        for p in ps {
            self.inner.0.push(self.get_position()? + p);
        }
        Ok(self)
    }

    pub fn close(mut self) -> Result<Polygon> {
        self.inner.remove_repeated_points_mut();
        ensure!(self.inner.0.len() >= 3, "Can only close a path with at least 3 points");
        self.inner.close();
        Ok(Polygon::new(self.inner, vec![]))
    }

    pub fn enthicken(mut self, thickness: f64) -> Result<ThickLineString> {
        self.inner.remove_repeated_points_mut();
        ensure!(self.inner.0.len() >= 2, "Can only enthicken a path with at least 2 points");
        ensure!(!self.inner.is_closed(), "Didn't expect a line to be closed");
        Ok(ThickLineString::new(self.inner, thickness))
    }
}


#[derive(Debug)]
pub struct SvgPrimitives {
    pub lines: Vec<ThickLineString>,
    pub polygons: Vec<Polygon>,
    pub circles: Vec<Circle>,
}

impl SvgPrimitives {
    pub fn new() -> Self {
        Self {
            lines: vec![],
            polygons: vec![],
            circles: vec![],
        }
    }

    fn add_line(&mut self, line_new: ThickLineString) {
        for line in &mut self.lines {
            if line.can_join(&line_new) {
                line.join(line_new);
                return;
            }
        }

        self.lines.push(line_new);
    }

    fn join_all_lines(&mut self) {
        let mut new_lines = vec![];

        let find_next = |lines: &Vec<ThickLineString>, current: &ThickLineString| lines.iter()
            .enumerate()
            .rev()
            .find_map(|(i, l)| if l.can_join(current) { Some(i) } else { None });

        while let Some(mut line) = self.lines.pop() {
            while let Some(i) = find_next(&self.lines, &line) {
                line.join(self.lines.remove(i));
            }

            new_lines.push(line);
        }

        self.lines = new_lines;
    }

    pub fn add_from_path(&mut self, ctx: &SvgContext, path_data: path::Data) -> Result<()> {
        let mut builder = PathBuilder::new();

        for command in path_data.iter() {
            use svg::node::element::path::{Command::*, Position::*};

            match command {
                | &Move(_, ref params)
                | &Line(_, ref params) => {
                    ensure!(params.len() % 2 == 0);
                    let pts = params.chunks(2).filter_map(|p| {
                        if let [x, y] = p {
                            Some(Coord{ x: *x as f64, y: *y as f64 })
                        } else {
                            None
                        }
                    });

                    match command {
                        Move(Absolute, ..) => builder = builder.moveto(pts)?,
                        Move(Relative, ..) => builder = builder.moveby(pts)?,
                        Line(Absolute, ..) => builder = builder.lineto(pts)?,
                        Line(Relative, ..) => builder = builder.lineby(pts)?,
                        _ => unreachable!(),
                    }
                },
                &VerticalLine(Absolute, ref params) => {
                    let x = builder.get_position()?.x;
                    builder = builder.lineto(params.iter().map(|y| Coord { x, y: *y as f64 }))?;
                },
                &VerticalLine(Relative, ref params) => {
                    builder = builder.lineby(params.iter().map(|y| Coord { x: 0.0, y: *y as f64 }))?;
                },
                &HorizontalLine(Absolute, ref params) => {
                    let y = builder.get_position()?.y;
                    builder = builder.lineto(params.iter().map(|x| Coord { x: *x as f64, y }))?;
                },
                &HorizontalLine(Relative, ref params) => {
                    builder = builder.lineby(params.iter().map(|x| Coord { x: *x as f64, y: 0.0 }))?;
                },
                &EllipticalArc(_, ref params) => {
                    ensure!(params.len() % 7 == 0);
                    warn!("Elliptical arc replaced with a straight line!");
                    let pts = params.chunks(7).filter_map(|p| {
                        if let [_, _, _, _, _, x, y] = p {
                            Some(Coord { x: *x as f64, y: *y as f64 })
                        } else {
                            None
                        }
                    });

                    match command {
                        EllipticalArc(Absolute, ..) => builder = builder.lineto(pts)?,
                        EllipticalArc(Relative, ..) => builder = builder.lineby(pts)?,
                        _ => unreachable!(),
                    }
                },
                &Close => {
                    let polygon = builder.close()?;
                    self.polygons.push(polygon);
                    return Ok(());
                },
                command => {
                    bail!("Unsupported path command {command:?}");
                },
            }
        }

        // TODO: handle stroke width set on the elements themselves

        let line = builder.enthicken(ctx.get_stroke_width()?)?;
        self.add_line(line);

        Ok(())
    }

    pub fn add_circle(&mut self, center: Coord, radius: f64) -> Result<()> {
        ensure!(radius > 0.0, "Circle radius should be greater than 0");
        self.circles.push(Circle::new(center, radius));
        Ok(())
    }

    pub fn holes(&self) -> Vec<Hole> {
        self.circles.iter()
            .map(|c| Hole::new(c.center, c.radius))
            .collect()
    }

    pub fn contours(mut self, resolution: f64) -> MultiPolygon {
        self.join_all_lines();

        self.circles.into_iter().map(|c| c.into_polygon(resolution))
            .chain(self.lines.into_iter().map(|l| l.into_polygon(resolution)))
            .chain(self.polygons.into_iter())
            .collect()
    }
}


pub fn process_svg(parser: Parser) -> Result<SvgPrimitives> {
    let mut ctx = SvgContext::new();
    let mut shapes = SvgPrimitives::new();

    for event in parser {
        match event {

            /* Ignore some events */

            | Event::Instruction(..)
            | Event::Declaration(..)
            | Event::Text(..)
            | Event::Comment(..)
            | Event::Tag(tag::SVG, ..)
            | Event::Tag(tag::Description, ..)
            | Event::Tag(tag::Text, ..)
            | Event::Tag(tag::Title, ..) => {},

            /* Handle group opening and closing */

            Event::Tag(tag::Group, tag::Type::Start, attrs) => {
                ctx.group_push(&attrs)?;
            },
            Event::Tag(tag::Group, tag::Type::End, ..) => {
                ctx.group_pop()?;
            }

            /* Handle paths */

            Event::Tag(tag::Path, tag::Type::Empty, ref attrs) => {
                let data = attrs.get("d").context("No 'd' on a path")?;
                let data = path::Data::parse(data)?;

                if let Err(err) = shapes.add_from_path(&ctx, data) {
                    error!("Error processing {event:?}, skipping it:\n{err}");
                }
            },

            /* Handle circles */

            Event::Tag(tag::Circle, tag::Type::Empty, attrs) => {
                let cx: f64 = attrs.get("cx").context("No 'cx' on circle")?.parse()?;
                let cy: f64 = attrs.get("cy").context("No 'cy' on circle")?.parse()?;
                let r: f64 = attrs.get("r").context("No 'r' on circle")?.parse()?;

                shapes.add_circle(Coord { x: cx, y: cy }, r)?;
            },

            /* Everything else is not supported */

            event => {
                warn!("Unsupported event {event:?}");
            }
        }
    }

    Ok(shapes)
}
