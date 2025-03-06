use anyhow::{ensure, Context, Result};
use nalgebra as na;
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use crate::geo::edge::{Edge, Turning};

use super::{Float, Point, TAU, PI};


pub struct Line {
    points: Vec<Point>,
}

impl Line {
    pub fn empty() -> Self {
        Self {
            points: vec![],
        }
    }

    pub fn do_move(&mut self, p: Point) -> Result<()> {
        ensure!(self.points.len() == 0, "Move is only supported as the first command");
        self.points.push(p);
        Ok(())
    }

    pub fn do_line(&mut self, p: Point) -> Result<()> {
        ensure!(self.points.len() > 0, "Line is only supported as a follow-up command");
        self.points.push(p);
        Ok(())
    }

    pub fn do_close(self) -> Result<Contour> {
        ensure!(self.points.len() >= 3, "Can only close an area with at least 3 points");
        Contour::new(self.points)
    }

    pub fn do_enthicken(self, thickness: Float, cap_steps: usize) -> Result<Contour> {
        ensure!(cap_steps > 0);
        let cap_rot = na::Rotation2::new(PI / cap_steps as f32);

        let pts = self.points.len();
        ensure!(pts >= 2, "A line should have at least 2 points");

        ensure!(thickness > 0.0, "Tried to set negative thickness?");

        let map_edge = |(p1, p2): (&Point, &Point)| Edge::new(p1.clone(), p2.clone()).translate_right(thickness / 2.0);

        let edge_first = Edge::new(self.points[0].clone(), self.points[1].clone());
        let edge_last = Edge::new(self.points[pts - 2].clone(), self.points[pts - 1].clone());

        let mut boundary = vec![];

        // Find start line cap

        let mut v_cap_start = edge_first.left().normalize() * thickness / 2.0;
        for _ in 0..=cap_steps {
            boundary.push(self.points[0] + v_cap_start);
            v_cap_start = cap_rot * v_cap_start;
        }

        // Find the right edge

        let mut edges_r = self.points.iter()
            .zip(self.points.iter().skip(1))
            .map(map_edge);

        let mut edge_prev = edges_r.next().context("Expected at least one segment")?;
        for edge in edges_r {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        // Find end line cap

        let mut v_cap_end = edge_last.right().normalize() * thickness / 2.0;
        for _ in 0..=cap_steps {
            boundary.push(self.points[pts - 1] + v_cap_end);
            v_cap_end = cap_rot * v_cap_end;
        }

        // Find the left edge

        let mut edges_l = self.points.iter().rev()
            .zip(self.points.iter().rev().skip(1))
            .map(map_edge);

        let mut edge_prev = edges_l.next().context("Expected at least one segment")?;
        for edge in edges_l {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        Contour::new(boundary)
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

    pub fn to_contour(self, sides: usize) -> Result<Contour> {
        ensure!(sides > 0);
        ensure!(self.radius > 0.0);

        let rot = na::Matrix3::new_rotation(TAU / sides as Float);
        let center = self.center.to_homogeneous();

        let mut boundary = vec![];
        let mut v = na::vector![0.0, self.radius, 1.0];
        for _ in 0..sides {
            let p = center + v;
            boundary.push(na::point![p.x, p.y]);
            v = rot * v;
        }

        Contour::new(boundary)
    }
}


/// Represents a contour defined by a closed loop of points, ordered counter-clockwise
pub struct Contour {
    boundary: Vec<Point>,
}

impl Contour {
    pub fn new(boundary: Vec<Point>) -> Result<Self> {
        ensure!(boundary.len() >= 3);

        let mut s = Self { boundary };

        let mut turned_left = false;
        let mut turned_right = false;

        for (e1, e2) in s.edge_pairs()? {
            match e1.turning(&e2.end) {
                Turning::Left => turned_left = true,
                Turning::Right => turned_right = true,
                Turning::Collinear => {},
            }
        }

        if turned_right && !turned_left {
            eprintln!("Contour winding is backwards. Please fix.");
            s.boundary.reverse();
        }

        Ok(s)
    }

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

    pub fn edge_pairs(&self) -> Result<impl Iterator<Item = (Edge, Edge)>> {
        let mut edges_a = self.edges()?.peekable();
        let e0 = std::iter::once(edges_a.peek().context("No edges?")?.clone());
        let edges_b = self.edges()?.skip(1).chain(e0);
        let edges = edges_a.into_iter().zip(edges_b);
        Ok(edges)
    }

    pub fn grow(&mut self, offset: Float) -> Result<()> {
        if offset == 0.0 {
            return Ok(());
        }

        ensure!(offset > 0.0);

        let mut edges = self.edges()?.map(|e| e.translate_right(offset)).peekable();
        let mut edge_prev = edges.peek().context("No edges?")?.clone();
        let edges = edges.skip(1).chain(std::iter::once(edge_prev.clone()));

        let mut boundary = vec![];

        for edge in edges {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        self.boundary = boundary;

        Ok(())
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

    pub fn is_convex(&self) -> Result<bool> {
        // Convex if never turning right

        let mut turned_right = false;

        for (e1, e2) in self.edge_pairs()? {
            match e1.turning(&e2.end) {
                Turning::Right => turned_right = true,
                _ => {},
            }
        }

        return Ok(!turned_right)
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

        let contour: Contour = line.do_enthicken(1.0, 1).unwrap();
        let points: Vec<_> = contour.points().unwrap().collect();

        assert_eq!(points.len(), 4);

        let d3 = distance(&points[0], &point![-0.5, 0.0]);
        let d0 = distance(&points[1], &point![0.5, 0.0]);
        let d1 = distance(&points[2], &point![0.5, 1.0]);
        let d2 = distance(&points[3], &point![-0.5, 1.0]);

        assert!(d0 < E, "{d0} is not close to 0");
        assert!(d1 < E, "{d1} is not close to 0");
        assert!(d2 < E, "{d2} is not close to 0");
        assert!(d3 < E, "{d3} is not close to 0");
    }
}
