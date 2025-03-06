use anyhow::{ensure, Context, Result};
use nalgebra as na;
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use crate::geo::edge::{Edge, Turning};

use super::{Float, Point, TAU};


pub struct Line {
    points: Vec<Point>,
}

impl Line {
    pub fn empty() -> Self {
        Self {
            points: vec![],
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

    #[must_use]
    pub fn do_close(self) -> Result<Contour> {
        ensure!(self.points.len() >= 3, "Can only close an area with at least 3 points");
        Ok(Contour {
            boundary: self.points,
        })
    }

    #[must_use]
    pub fn do_enthicken(self, thickness: Float) -> Result<Contour> {
        let pts = self.points.len();
        ensure!(pts >= 2, "A line should have at least 2 points");

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

        let mut boundary = vec![];

        // Find all the boundary points

        boundary.push(p_first);

        let mut edge_prev = edges_r.next().context("Expected at least one segment")?;
        for edge in edges_r {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        boundary.push(p_half1);
        boundary.push(p_half2);

        let mut edge_prev = edges_l.next().context("Expected at least one segment")?;
        for edge in edges_l {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        boundary.push(p_last);

        Ok(Contour {
            boundary,
        })
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

impl TryInto<Contour> for Circle {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<Contour, Self::Error> {
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

        let mut boundary = vec![];
        let mut v = na::vector![0.0, self.radius, 1.0];
        for _ in 0..sides {
            let p = center + v;
            boundary.push(na::point![p.x, p.y]);
            v = rot * v;
        }

        Ok(Contour {
            boundary
        })
    }
}


/// Represents a contour defined by a closed loop of points, ordered counter-clockwise
pub struct Contour {
    boundary: Vec<Point>,
    // TODO: support additional offset
}

impl Contour {
    pub fn points(&self) -> Result<impl DoubleEndedIterator<Item = Point>> {
        Ok(self.boundary.clone().into_iter())
    }

    pub fn edges(&self) -> Result<impl Iterator<Item = Edge>> {
        let mut edge_starts = self.points()?.peekable();
        let p0 = std::iter::once(edge_starts.peek().context("No points?")?.clone());
        let edge_ends = self.points()?.skip(1).chain(p0);
        let edges = edge_starts.into_iter().zip(edge_ends).map(|(p1, p2)| Edge::new(p1, p2));
        Ok(edges)
    }

    pub fn is_convex(&self) -> Result<bool> {
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


#[cfg(test)]
mod tests {
    use nalgebra::{point, distance};

    use crate::geo::E;

    use super::*;

    #[test]
    fn line_points() {
        let mut line = Line::empty();

        line.do_move(point![0.0, 0.0]).unwrap();
        line.do_line(point![0.0, 1.0]).unwrap();

        let contour: Contour = line.do_enthicken(1.0).unwrap();
        let points: Vec<_> = contour.points().unwrap().collect();

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
