use anyhow::{bail, ensure, Context, Result};
use nalgebra as na;

use crate::{feq, geo::{contour::Contour, edge::Edge}, p2eq};

use super::{edge::Turning, Float, Point, PI, TAU};


pub trait Shape {
    fn set_resolution(&mut self, resolution: Option<Float>) -> Result<()>;
    fn grow(&mut self, offset: Float) -> Result<()>;
    fn as_contour(&self) -> Result<Contour>;
}


pub struct PathBuilder {
    points: Vec<Point>,
}

impl PathBuilder {
    pub fn new() -> Self {
        Self {
            points: vec![],
        }
    }

    pub fn add_moveto(&mut self, p: Point) -> Result<()> {
        ensure!(self.points.len() == 0, "Move is only supported as the first command");
        self.points.push(p);
        Ok(())
    }

    pub fn do_moveto(mut self, p: Point) -> Result<Self> {
        self.add_moveto(p)?;
        Ok(self)
    }

    pub fn add_lineto(&mut self, p: Point) -> Result<()> {
        ensure!(self.points.len() > 0, "Line is only supported as a follow-up command");
        self.points.push(p);
        Ok(())
    }

    pub fn do_lineto(mut self, p: Point) -> Result<Self> {
        self.add_lineto(p)?;
        Ok(self)
    }

    pub fn into_convex_polygon(self) -> Result<ConvexPolygon> {
        ConvexPolygon::new(self.points)
    }

    pub fn into_line(self, thickness: Float) -> Result<Line> {
        Line::new(self.points, thickness)
    }
}


pub struct Line {
    points: Vec<Point>,
    thickness: Float,
    resolution: Option<Float>,
}

impl Line {
    pub fn new(points: Vec<Point>, thickness: Float) -> Result<Self> {
        ensure!(points.len() >= 2, "A line should have at least 2 points");
        ensure!(thickness > 0.0, "Thickness must be greater than 0");
        Ok(Self {
            points,
            thickness,
            resolution: None,
        })
    }

    pub fn point_first(&self) -> &Point {
        assert!(self.points.len() >= 2);
        &self.points[0]
    }

    pub fn point_last(&self) -> &Point {
        assert!(self.points.len() >= 2);
        &self.points[self.points.len() - 1]
    }

    /// Try to connect `other` to `self`.
    /// Return `other` if could not connect.
    pub fn try_merge(&mut self, other: Self) -> Option<Self> {
        if !feq!(self.thickness, other.thickness) {
            // Can only join lines if they have the same thickness
            return Some(other);
        }

        let s_first = self.point_first();
        let s_last = self.point_last();
        let o_first = other.point_first();
        let o_last = other.point_last();

        if p2eq!(s_first, o_first) {
            self.points.reverse();
            self.points.extend(other.points.into_iter().skip(1));
            return None;
        }

        if p2eq!(s_first, o_last) {
            self.points.reverse();
            self.points.extend(other.points.into_iter().rev().skip(1));
            return None;
        }

        if p2eq!(s_last, o_first) {
            self.points.extend(other.points.into_iter().skip(1));
            return None;
        }

        if p2eq!(s_last, o_last) {
            self.points.extend(other.points.into_iter().rev().skip(1));
            return None;
        }

        Some(other)
    }
}

impl Shape for Line {
    fn set_resolution(&mut self, resolution: Option<Float>) -> Result<()> {
        if let Some(resolution) = resolution {
            ensure!(resolution > 0.0);
        }
        self.resolution = resolution;
        Ok(())
    }

    fn grow(&mut self, offset: Float) -> Result<()> {
        ensure!(offset >= 0.0);
        self.thickness += offset * 2.0;
        Ok(())
    }

    fn as_contour(&self) -> Result<Contour> {
        let Line {
            points,
            thickness,
            resolution,
        } = self;

        let resolution = resolution.unwrap_or(1.0);

        let cap_circumference = PI * self.thickness / 2.0;
        let cap_segments = (cap_circumference / resolution).ceil() as usize;
        let cap_rot = na::Rotation2::new(PI / cap_segments as f32);

        let map_edge = |(p1, p2): (&Point, &Point)| Edge::from((p1, p2)).translate_right(thickness / 2.0);

        let edge_first = Edge::from((points[0], points[1]));
        let edge_last = Edge::from((points[points.len() - 2], points[points.len() - 1]));

        let mut boundary = vec![];

        // Find start line cap

        let mut v_cap_start = edge_first.left().normalize() * (thickness / 2.0);
        for _ in 0..=cap_segments {
            boundary.push(points[0] + v_cap_start);
            v_cap_start = cap_rot * v_cap_start;
        }

        // Find the right edge

        let mut edges_r = points.iter()
            .zip(points.iter().skip(1))
            .map(map_edge);

        let mut edge_prev = edges_r.next().context("Expected at least one segment")?;
        for edge in edges_r {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        // Find end line cap

        let mut v_cap_end = edge_last.right().normalize() * (thickness / 2.0);
        for _ in 0..=cap_segments {
            boundary.push(points[points.len() - 1] + v_cap_end);
            v_cap_end = cap_rot * v_cap_end;
        }

        // Find the left edge

        let mut edges_l = points.iter().rev()
            .zip(points.iter().rev().skip(1))
            .map(map_edge);

        let mut edge_prev = edges_l.next().context("Expected at least one segment")?;
        for edge in edges_l {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        Ok(Contour::from_ccwise_boundary(boundary))
    }
}


pub struct ConvexPolygon {
    /// A closed loop of points, ordered counter-clockwise
    boundary: Vec<Point>,
}

impl ConvexPolygon {
    pub fn new(boundary: Vec<Point>) -> Result<Self> {
        ensure!(boundary.len() >= 3, "Need at least 3 points for a polygon");

        let mut s = Self { boundary };

        let mut turned_left = false;
        let mut turned_right = false;

        for (e1, e2) in s.edge_pairs()? {
            match e1.turning(e2.end()) {
                Turning::Left => turned_left = true,
                Turning::Right => turned_right = true,
                Turning::Collinear => {},
            }
        }

        if turned_right && !turned_left {
            eprintln!("Boundary winding is backwards. Please fix.");
            s.boundary.reverse();
        } else if turned_right {
            bail!("Boundary is not convex.");
        }

        Ok(s)
    }

    pub fn points(&self) -> Result<impl DoubleEndedIterator<Item = &Point>> {
        Ok(self.boundary.iter())
    }

    pub fn edges(&self) -> Result<impl Iterator<Item = Edge>> {
        let mut edge_starts = self.points()?.peekable();
        let p0 = std::iter::once(*edge_starts.peek().context("No points?")?);
        let edge_ends = self.points()?.skip(1).chain(p0);
        let edges = edge_starts.into_iter().zip(edge_ends).map(|(p1, p2)| Edge::from((p1, p2)));
        Ok(edges)
    }

    pub fn edge_pairs(&self) -> Result<impl Iterator<Item = (Edge, Edge)>> {
        let mut edges_a = self.edges()?.peekable();
        let e0 = std::iter::once(edges_a.peek().context("No edges?")?.clone());
        let edges_b = self.edges()?.skip(1).chain(e0);
        let edges = edges_a.into_iter().zip(edges_b);
        Ok(edges)
    }
}

impl Shape for ConvexPolygon {
    fn set_resolution(&mut self, _resolution: Option<Float>) -> Result<()> {
        Ok(())
    }

    fn grow(&mut self, offset: Float) -> Result<()> {
        if offset == 0.0 {
            return Ok(());
        }

        ensure!(offset > 0.0);

        // TODO: delete edges when things become self-intersecting
        // TODO: rounded links?

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

    fn as_contour(&self) -> Result<Contour> {
        Ok(Contour::from_ccwise_boundary(self.boundary.clone()))
    }
}


pub struct Circle {
    center: Point,
    radius: Float,
    resolution: Option<Float>,
}

impl Circle {
    pub fn new(center: Point, radius: Float) -> Self {
        Self {
            center,
            radius,
            resolution: None,
        }
    }
}


impl Shape for Circle {
    fn set_resolution(&mut self, resolution: Option<Float>) -> Result<()> {
        if let Some(resolution) = resolution {
            ensure!(resolution > 0.0);
        }
        self.resolution = resolution;
        Ok(())
    }

    fn grow(&mut self, offset: Float) -> Result<()> {
        ensure!(offset >= 0.0);
        self.radius += offset;
        Ok(())
    }

    fn as_contour(&self) -> Result<Contour> {
        let Circle {
            center,
            radius,
            resolution,
        } = self;

        let resolution = resolution.unwrap_or(1.0);

        let circumference = TAU * radius;
        let segments = (circumference / resolution).ceil() as usize;
        let rot = na::Rotation2::new(TAU / segments as f32);

        let mut boundary = vec![];
        let mut v = na::vector![0.0, self.radius];
        for _ in 0..segments {
            let p = center + v;
            boundary.push(na::point![p.x, p.y]);
            v = rot * v;
        }

        Ok(Contour::from_ccwise_boundary(boundary))
    }
}


#[cfg(test)]
mod tests {
    use nalgebra::{point, distance};

    use crate::geo::E;

    use super::*;

    #[test]
    fn line_points() -> Result<()> {
        let mut line = PathBuilder::new()
            .do_moveto(point![0.0, 0.0])?
            .do_lineto(point![0.0, 1.0])?
            .into_line(1.0)?;

        line.set_resolution(Some(5.0))?;

        let contour: Contour = line.as_contour()?;
        let points: Vec<_> = contour.points()?.collect();

        ensure!(points.len() == 4);

        let d3 = distance(&points[0], &point![-0.5, 0.0]);
        let d0 = distance(&points[1], &point![0.5, 0.0]);
        let d1 = distance(&points[2], &point![0.5, 1.0]);
        let d2 = distance(&points[3], &point![-0.5, 1.0]);

        ensure!(d0 < E, "d0 {d0} is not close to 0");
        ensure!(d1 < E, "d1 {d1} is not close to 0");
        ensure!(d2 < E, "d2 {d2} is not close to 0");
        ensure!(d3 < E, "d3 {d3} is not close to 0");

        Ok(())
    }
}
