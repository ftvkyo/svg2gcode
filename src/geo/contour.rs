use anyhow::{Context, Result};
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use super::Point;


pub struct Contour {
    /// A closed loop of points, ordered counter-clockwise
    boundary: Vec<Point>,
}

impl Contour {
    pub fn from_ccwise_boundary(boundary: Vec<Point>) -> Self {
        Self {
            boundary
        }
    }

    pub fn points(&self) -> Result<impl DoubleEndedIterator<Item = &Point>> {
        Ok(self.boundary.iter())
    }

    pub fn svg(&self) -> Result<SvgPath> {
        let mut points = self.points()?.peekable();
        let first = points.peek().context("Expected at least 1 point")?;

        let mut data = SvgPathData::new()
            .move_to((first.x, first.y));

        for p in points.skip(1) {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        let path = SvgPath::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke");

        Ok(path)
    }
}
