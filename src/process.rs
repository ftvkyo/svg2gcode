use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use log::warn;
use nalgebra::point;
use svg::{node::element::{path::{Command, Data, Position}, tag, Group, Path}, parser::Event, Document};

use crate::{geo::{contour::{self, Contour, Contours}, shape::{self, PathBuilder, Shape, ShapeE}, Float, Point}, Args};


struct SvgContext {
    stroke_width: Vec<Option<Float>>,
}

impl SvgContext {
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
        assert!(self.stroke_width.pop().is_some());
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

struct Shapes {
    lines: Vec<shape::Line>,
    polygons: Vec<shape::ConvexPolygon>,
    circles: Vec<shape::Circle>,

    resolution_lines: Float,
    resolution_polygons: Float,
    resolution_circles: Float,
}

impl Shapes {
    pub fn new(args: &Args) -> Self {
        Self {
            lines: vec![],
            polygons: vec![],
            circles: vec![],

            resolution_lines: args.resolution_lines.unwrap_or(1.0),
            resolution_polygons: args.resolution_lines.unwrap_or(1.0),
            resolution_circles: args.resolution_circles.unwrap_or(1.0),
        }
    }

    pub fn grow(&mut self, amount: Float) {
        for line in &mut self.lines {
            line.grow(amount);
        }
        for poly in &mut self.polygons {
            poly.grow(amount);
        }
        for circ in &mut self.circles {
            circ.grow(amount);
        }
    }

    pub fn contours(self) -> impl Iterator<Item = Contour> {
        let Self {
            lines,
            polygons,
            circles,
            ..
        } = self;

        lines.into_iter().map(|s| ShapeE::Line(s))
            .chain(polygons.into_iter().map(|s| ShapeE::Poly(s)))
            .chain(circles.into_iter().map(|s| ShapeE::Circ(s)))
            .map(Contour::new)
    }

    pub fn add_from_path(&mut self, ctx: &SvgContext, path_data: Data) -> Result<()> {
        let mut builder = PathBuilder::new();

        for command in path_data.iter() {
            use Command::*;
            use Position::*;

            match command {
                | &Move(_, ref params)
                | &Line(_, ref params) => {
                    ensure!(params.len() % 2 == 0);
                    let pts = params.chunks(2).filter_map(|p| {
                        if let [x, y] = p {
                            Some(point![*x, *y])
                        } else {
                            None
                        }
                    });

                    match command {
                        Move(Absolute, ..) => builder.add_moveto(pts)?,
                        Move(Relative, ..) => builder.add_moveby(pts)?,
                        Line(Absolute, ..) => builder.add_lineto(pts)?,
                        Line(Relative, ..) => builder.add_lineby(pts)?,
                        _ => unreachable!(),
                    }
                },
                &VerticalLine(Absolute, ref params) => {
                    let x = builder.get_position().x;
                    builder.add_lineto(params.iter().map(|y| point![x, *y]))?;
                },
                &VerticalLine(Relative, ref params) => {
                    builder.add_lineby(params.iter().map(|y| point![0.0, *y]))?;
                },
                &HorizontalLine(Absolute, ref params) => {
                    let y = builder.get_position().y;
                    builder.add_lineto(params.iter().map(|x| point![*x, y]))?;
                },
                &HorizontalLine(Relative, ref params) => {
                    builder.add_lineby(params.iter().map(|x| point![*x, 0.0]))?;
                },
                &EllipticalArc(_, ref params) => {
                    ensure!(params.len() % 7 == 0);
                    warn!("Elliptical arc replaced with a straight line!");
                    let pts = params.chunks(7).filter_map(|p| {
                        if let [_, _, _, _, _, x, y] = p {
                            Some(point![*x, *y])
                        } else {
                            None
                        }
                    });

                    match command {
                        EllipticalArc(Absolute, ..) => builder.add_lineto(pts)?,
                        EllipticalArc(Relative, ..) => builder.add_lineby(pts)?,
                        _ => unreachable!(),
                    }
                },
                &Close => {
                    let polygon = builder.into_convex_polygon(self.resolution_polygons)?;
                    self.polygons.push(polygon);
                    return Ok(());
                },
                command => {
                    bail!("Unsupported path command {command:?}");
                },
            }
        }

        let mut line = builder.into_line(ctx.get_stroke_width()?, self.resolution_lines)?;

        for existing in &mut self.lines {
            if let Some(unmerged) = existing.try_merge(line) {
                line = unmerged;
            } else {
                return Ok(());
            }
        }

        self.lines.push(line);

        Ok(())
    }

    pub fn add_circle(&mut self, center: Point, radius: Float) {
        let circle = shape::Circle::new(center, radius, self.resolution_circles);
        self.circles.push(circle);
    }
}


pub fn process(args: &Args) -> Result<(Contours, Group)> {
    let mut ctx = SvgContext::new();

    let mut g_originals = Group::new()
        .set("opacity", "50%");

    let mut shapes = Shapes::new(args);

    let mut content = String::new();
    for event in svg::open(&args.input, &mut content)? {
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

            Event::Tag(tag::Path, tag::Type::Empty, attrs) => {
                let data = attrs.get("d")
                    .context("No 'd' attribute on a path?")?;
                let data = Data::parse(data)?;

                shapes.add_from_path(&ctx, data)?;

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

                shapes.add_circle(point![cx, cy], r);

                // Save the original shape too

                let mut original = svg::node::element::Circle::new();
                original = fix_attributes(original, attrs.clone())?;
                g_originals = g_originals.add(original);
            },

            /* Everything else is not supported */

            event => {
                warn!("Unsupported event {event:?}");
            }
        }
    }

    shapes.grow(args.offset);

    let mut contours = contour::Contours::from(shapes.contours());
    contours.merge_all();

    Ok((contours, g_originals))
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
    use svg::node::Value;

    let none = Value::from("none");
    let attrs = node.get_attributes_mut().context("No attributes?")?;

    let mut stroke = attrs.get("stroke").cloned();

    let style = attrs.get("style");
    if let Some(style) = style {
        for prop in style.split(";") {
            if let Some((prop_key, prop_val)) = prop.split_once(":") {
                let prop_key = prop_key.trim();
                let prop_val = prop_val.trim();

                match prop_key {
                    "stroke" => stroke = Some(Value::from(prop_val)),
                    _ => {},
                }
            }
        }
    }

    if !attrs.contains_key("stroke-width") && stroke != Some(none) {
        attrs.insert("stroke-width".to_string(), get_stroke_width()?.into());
    }

    Ok(node)
}


fn make_gizmo(size: Float) -> Group {
    use svg::node::element;

    let circle = element::Circle::new()
        .set("cx", 0)
        .set("cy", 0)
        .set("r", size / 2.0)
        .set("fill", "none")
        .set("stroke", "blue")
        .set("stroke-width", 5)
        .set("vector-effect", "non-scaling-stroke")
        .set("opacity", "25%");

    let x_axis = element::Line::new()
        .set("x1", 0)
        .set("y1", 0)
        .set("x2", size)
        .set("y2", 0)
        .set("stroke", "red")
        .set("stroke-width", 1)
        .set("vector-effect", "non-scaling-stroke");

    let y_axis = element::Line::new()
        .set("x1", 0)
        .set("y1", 0)
        .set("x2", 0)
        .set("y2", size)
        .set("stroke", "green")
        .set("stroke-width", 1)
        .set("vector-effect", "non-scaling-stroke");

    Group::new()
        .add(circle)
        .add(x_axis)
        .add(y_axis)
}


pub fn make_svg(contours: Contours, g_originals: Group) -> Result<svg::Document> {
    let mut g_contours = Group::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1);

    let mut min_x: Float = 0.0;
    let mut max_x: Float = 0.0;
    let mut min_y: Float = 0.0;
    let mut max_y: Float = 0.0;

    for contour in contours.contours {
        for point in contour.points() {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        g_contours = g_contours.add(contour.svg());
    }

    let mut g_problems = Group::new()
        .set("fill", "none")
        .set("stroke", "red")
        .set("stroke-width", 1);

    for problem in contours.problems {
        let start = problem.start();
        let end = problem.end();
        let data = Data::new()
            .move_to((start.x, start.y))
            .line_to((end.x, end.y));
        let path = Path::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke");
        g_problems = g_problems.add(path);
    }

    let gizmo_size: Float = 5.0;

    min_x -= gizmo_size;
    max_x += gizmo_size;
    min_y -= gizmo_size;
    max_y += gizmo_size;

    let document = Document::new()
        .add(g_originals)
        .add(g_contours)
        .add(g_problems)
        .add(make_gizmo(gizmo_size))
        .set("viewBox", (min_x, min_y, max_x - min_x, max_y - min_y));

    Ok(document)
}
