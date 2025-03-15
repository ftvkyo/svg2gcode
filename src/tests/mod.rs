mod concept;
mod generated;
mod stress;

use std::path::Path;

use anyhow::{ensure, Result};
use geo::Coord;
use svg::node::element;

use crate::{input::process_svg, output::make_svg, transform::polygons_unite};

pub const OUTDIR: &'_ str = "tmp/test-output/";

fn ensure_dir(dir: impl AsRef<Path>) -> Result<()> {
    let dir = dir.as_ref();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    ensure!(dir.is_dir(), "{dir:?} should be a directory");
    Ok(())
}

pub fn init_test_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp(None)
        .format_target(false)
        .is_test(true)
        .try_init();
}

pub fn run(name: &str, doc: &svg::Document, offset: Option<f64>) -> Result<()> {
    init_test_logger();
    ensure_dir(&OUTDIR)?;

    let input = format!("input-{name}");
    let input = Path::new(OUTDIR).join(input).with_extension("svg");

    let output = format!("output-{name}");
    let output = Path::new(OUTDIR).join(output).with_extension("svg");

    svg::save(&input, doc)?;
    let mut content = String::new();
    let parser = svg::open(&input, &mut content)?;

    let shapes = process_svg(parser)?;
    let polygons = shapes.polygons(offset.unwrap_or(0.0), 0.1);
    let polygons = polygons_unite(polygons);

    let doc = make_svg(polygons, vec![]);

    svg::save(output, &doc)?;

    Ok(())
}

pub fn make_line(points: Vec<Coord>, thickness: f64) -> element::Group {
    let mut data = element::path::Data::new()
        .move_to(points[0].x_y());

    for point in points.into_iter().skip(1) {
        data = data.line_to(point.x_y());
    }

    let path = element::Path::new()
        .set("d", data)
        .set("fill", "none")
        .set("stroke", "black");

    element::Group::new()
        .set("style", format!("stroke-width: {thickness}"))
        .add(path)
}

pub fn make_polygon(points: Vec<Coord>) -> element::Path {
    let mut data = element::path::Data::new()
        .move_to(points[0].x_y());

    for point in points.into_iter().skip(1) {
        data = data.line_to(point.x_y());
    }

    data = data.close();

    element::Path::new()
        .set("d", data)
        .set("fill", "black")
        .set("stroke", "none")
}

pub fn make_circle(center: Coord, radius: f64) -> element::Circle {
    element::Circle::new()
        .set("cx", center.x)
        .set("cy", center.y)
        .set("r", radius)
        .set("fill", "black")
        .set("stroke", "none")
}

pub fn make_test_svg(shapes: element::Group, dim: (usize, usize)) -> svg::Document {
    svg::Document::new()
        .set("viewBox", (0, 0, dim.0, dim.1))
        .add(shapes)
}
