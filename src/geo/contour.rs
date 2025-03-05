use anyhow::{ensure, Context, Result};
use nalgebra as na;
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use crate::geo::edge::{Edge, Turning};

use super::{Float, Point, TAU};


/// Represents a contour defined by a closed loop of points, ordered counter-clockwise
pub trait Contour {
    // TODO: pass additional offset as an argument
    fn points(&self) -> Result<impl DoubleEndedIterator<Item = Point>>;

    fn edges(&self) -> Result<impl Iterator<Item = Edge>> {
        let mut edge_starts = self.points()?.peekable();
        let p0 = std::iter::once(edge_starts.peek().context("No points?")?.clone());
        let edge_ends = self.points()?.skip(1).chain(p0);
        let edges = edge_starts.into_iter().zip(edge_ends).map(|(p1, p2)| Edge::new(p1, p2));
        Ok(edges)
    }

    fn is_convex(&self) -> Result<bool> {
        let mut edges = self.edges()?.peekable();

        let first = edges.peek().context("No points?")?.clone();
        let mut prev = first.clone();

        for edge in edges {
            if prev.turning(&edge.end) == Turning::Right {
                return Ok(false);
            }

            prev = edge;
        }

        if prev.turning(&first.end) == Turning::Right {
            return Ok(false);
        }

        Ok(true)
    }

    fn svg(&self) -> Result<SvgPath> {
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


pub struct Line {
    points: Vec<Point>,
    thickness: Option<Float>,
}

impl Line {
    pub fn empty() -> Self {
        Self {
            points: vec![],
            thickness: None,
        }
    }

    #[must_use]
    pub fn do_move(&mut self, p: Point) -> Result<()> {
        ensure!(self.points.len() == 0, "Move is only supported as the first command");
        self.points.push(p);
        Ok(())
    }

    #[must_use]
    pub fn do_line(&mut self, p: Point) -> Result<()> {
        ensure!(self.points.len() > 0, "Line is only supported as a follow-up command");
        self.points.push(p);
        Ok(())
    }

    pub fn set_thickness(&mut self, thickness: Float) {
        self.thickness = Some(thickness);
    }

    #[must_use]
    pub fn do_close(self) -> Result<Area> {
        ensure!(self.points.len() >= 3, "Can only close an area with at least 3 points");
        ensure!(self.thickness.is_none(), "Can only close an area with strokes of thickness 0");
        Ok(Area {
            boundary: self.points,
        })
    }
}

impl Contour for Line {
    // TODO: line caps in lines should (maybe) respond to thickness
    fn points(&self) -> Result<impl DoubleEndedIterator<Item = Point>> {
        let pts = self.points.len();
        ensure!(pts >= 2, "A line should have at least 2 points");

        let thickness = self.thickness.context("No thickness set for a line?")?;
        ensure!(thickness > 0.0, "Tried to set negative thickness?");

        let map_edge = |(p1, p2): (&Point, &Point)| Edge::new(p1.clone(), p2.clone()).translate_right(thickness / 2.0);

        let edge_first = Edge::new(self.points[0].clone(), self.points[1].clone());
        let edge_last = Edge::new(self.points[pts - 2].clone(), self.points[pts - 1].clone());

        // Calculate the terminal points

        let p_first = edge_first.translate_right(thickness / 2.0).start;
        let p_half1 = edge_last.translate_right(thickness / 2.0).end;
        let p_half2 = edge_last.translate_right(- thickness / 2.0).end;
        let p_last = edge_first.translate_right(- thickness / 2.0).start;

        let mut edges_r = self.points.iter()
            .zip(self.points.iter().skip(1))
            .map(map_edge);
        let mut edges_l = self.points.iter().rev()
            .zip(self.points.iter().rev().skip(1))
            .map(map_edge);

        let mut points = vec![];

        // Find all the boundary points

        points.push(p_first);

        let mut edge_prev = edges_r.next().context("Expected at least one segment")?;
        for edge in edges_r {
            points.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        points.push(p_half1);
        points.push(p_half2);

        let mut edge_prev = edges_l.next().context("Expected at least one segment")?;
        for edge in edges_l {
            points.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        points.push(p_last);

        Ok(points.into_iter())
    }
}


pub struct Area {
    boundary: Vec<Point>,
}

impl Contour for Area {
    fn points(&self) -> Result<impl DoubleEndedIterator<Item = Point>> {
        Ok(self.boundary.clone().into_iter())
    }
}


pub struct Circle {
    center: Point,
    radius: Float,
}

impl Circle {
    pub fn new(center: Point, radius: Float) -> Self {
        Self {
            center,
            radius,
        }
    }
}

impl Contour for Circle {
    fn points(&self) -> Result<impl DoubleEndedIterator<Item = Point>> {
        ensure!(self.radius > 0.0);

        let sides = if self.radius < 0.5 {
            12
        } else if self.radius < 2.0 {
            36
        } else {
            72
        };

        let rot = na::Matrix3::new_rotation(TAU / sides as Float);
        let center = self.center.to_homogeneous();

        let mut points = vec![];
        let mut v = na::vector![0.0, self.radius, 1.0];
        for _ in 0..sides {
            let p = center + v;
            points.push(na::point![p.x, p.y]);
            v = rot * v;
        }

        Ok(points.into_iter())
    }
}


#[cfg(test)]
mod tests {
    use nalgebra::{point, distance};

    use crate::geo::E;

    use super::*;

    #[test]
    fn line_points() {
        let mut line = Line::empty();

        line.set_thickness(1.0);
        line.do_move(point![0.0, 0.0]).unwrap();
        line.do_line(point![0.0, 1.0]).unwrap();

        let points: Vec<_> = line.points().unwrap().collect();

        assert_eq!(points.len(), 4);

        let d0 = distance(&points[0], &point![0.5, 0.0]);
        let d1 = distance(&points[1], &point![0.5, 1.0]);
        let d2 = distance(&points[2], &point![-0.5, 1.0]);
        let d3 = distance(&points[3], &point![-0.5, 0.0]);

        assert!(d0 < E, "{d0} is not close to 0");
        assert!(d1 < E, "{d1} is not close to 0");
        assert!(d2 < E, "{d2} is not close to 0");
        assert!(d3 < E, "{d3} is not close to 0");
    }
}
