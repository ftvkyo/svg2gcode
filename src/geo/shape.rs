use anyhow::{bail, ensure, Result};
use log::info;
use nalgebra as na;

use crate::{feq, geo::edge::Edge, p2eq};

use super::{edge::Turning, Float, Point, PI, TAU};


pub trait Shape {
    fn set_resolution(&mut self, resolution: Option<Float>);
    fn grow(&mut self, offset: Float);
    fn boundary(&self) -> Vec<Point>;
    fn contains(&self, p: &Point) -> bool;
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

    pub fn get_position(&self) -> Point {
        self.points.last().cloned().unwrap_or(na::point![0.0, 0.0])
    }

    pub fn add_moveto(&mut self, pts: impl Iterator<Item = Point>) -> Result<()> {
        ensure!(self.points.len() == 0, "Move is only supported as the first command");
        self.points.extend(pts);
        Ok(())
    }

    pub fn add_moveby(&mut self, pts: impl Iterator<Item = Point>) -> Result<()> {
        ensure!(self.points.len() == 0, "Move is only supported as the first command");

        for p in pts {
            self.points.push(self.get_position() + na::vector![p.x, p.y]);
        }

        Ok(())
    }

    pub fn do_moveto(mut self, p: Point) -> Result<Self> {
        self.add_moveto(std::iter::once(p))?;
        Ok(self)
    }

    pub fn add_lineto(&mut self, pts: impl Iterator<Item = Point>) -> Result<()> {
        ensure!(self.points.len() > 0, "Line is only supported as a follow-up command");
        self.points.extend(pts);
        Ok(())
    }

    pub fn add_lineby(&mut self, pts: impl Iterator<Item = Point>) -> Result<()> {
        ensure!(self.points.len() > 0, "Line is only supported as a follow-up command");

        for p in pts {
            self.points.push(self.get_position() + na::vector![p.x, p.y]);
        }

        Ok(())
    }

    pub fn do_lineto(mut self, p: Point) -> Result<Self> {
        self.add_lineto(std::iter::once(p))?;
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

    pub fn segments(&self) -> impl Iterator<Item = Edge> {
        assert!(self.points.len() >= 2);
        let edge_starts = self.points.iter();
        let edge_ends = self.points[1..].iter();
        edge_starts.zip(edge_ends).map(|(p1, p2)| Edge::from((p1, p2)))
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
    fn set_resolution(&mut self, resolution: Option<Float>) {
        self.resolution = resolution;
    }

    fn grow(&mut self, offset: Float) {
        self.thickness += offset * 2.0;
    }

    fn boundary(&self) -> Vec<Point> {
        let Line {
            points,
            thickness,
            resolution,
        } = self;

        let resolution = resolution.unwrap_or(1.0);

        let cap_radius = self.thickness / 2.0;
        let cap_circumference = PI * cap_radius;
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

        let mut edge_prev = edges_r.next().expect("At least one segment");
        for edge in edges_r {
            if edge_prev.crosses(&edge) {
                boundary.push(edge_prev.find_intersection(&edge));
            } else {
                boundary.extend(edge_prev.find_arc(&edge, thickness / 2.0, resolution))
            }

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

        let mut edge_prev = edges_l.next().expect("At least one segment");
        for edge in edges_l {
            if edge_prev.crosses(&edge) {
                boundary.push(edge_prev.find_intersection(&edge));
            } else {
                boundary.extend(edge_prev.find_arc(&edge, thickness / 2.0, resolution))
            }

            edge_prev = edge;
        }

        boundary
    }

    fn contains(&self, p: &Point) -> bool {
        for seg in self.segments() {
            if seg.distance(p) <= self.thickness / 2.0 {
                return true;
            }
        }

        false
    }
}


pub struct ConvexPolygon {
    /// A closed loop of points, ordered counter-clockwise
    boundary: Vec<Point>,
    resolution: Option<Float>,
}

impl ConvexPolygon {
    pub fn new(boundary: Vec<Point>) -> Result<Self> {
        ensure!(boundary.len() >= 3, "Need at least 3 points for a polygon");

        let mut s = Self {
            boundary,
            resolution: None,
        };

        let mut turned_left = false;
        let mut turned_right = false;

        for (e1, e2) in get_boundary_edge_pairs(&s.boundary) {
            match e1.turning(e2.end()) {
                Turning::Left => turned_left = true,
                Turning::Right => turned_right = true,
                Turning::Collinear => {},
            }
        }

        if turned_right && !turned_left {
            info!("Boundary winding is backwards. Fixing.");
            s.boundary.reverse();
        } else if turned_right {
            bail!("Boundary is not convex.");
        }

        Ok(s)
    }

    pub fn points(&self) -> Result<impl DoubleEndedIterator<Item = &Point>> {
        Ok(self.boundary.iter())
    }
}

impl Shape for ConvexPolygon {
    fn set_resolution(&mut self, resolution: Option<Float>) {
        self.resolution = resolution;
    }

    fn grow(&mut self, offset: Float) {
        if offset == 0.0 {
            return;
        }

        let resolution = self.resolution.unwrap_or(1.0);

        // TODO: delete edges when things become self-intersecting

        let mut edges = get_boundary_edges(&self.boundary).map(|e| e.translate_right(offset)).peekable();
        let mut edge_prev = edges.peek().expect("Edges").clone();
        let edges = edges.skip(1).chain(std::iter::once(edge_prev.clone()));

        let mut boundary = vec![];

        for edge in edges {
            boundary.extend(edge_prev.find_arc(&edge, offset, resolution));
            edge_prev = edge;
        }

        self.boundary = boundary;
    }

    fn boundary(&self) -> Vec<Point> {
        self.boundary.clone()
    }

    fn contains(&self, p: &Point) -> bool {
        get_boundary_edges(&self.boundary).all(|e| e.turning(p) != Turning::Right)
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
    fn set_resolution(&mut self, resolution: Option<Float>) {
        self.resolution = resolution;
    }

    fn grow(&mut self, offset: Float) {
        self.radius += offset;
    }

    fn boundary(&self) -> Vec<Point> {
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

        boundary
    }

    fn contains(&self, p: &Point) -> bool {
        (self.center - p).magnitude() <= self.radius
    }
}


pub fn get_boundary_edges(boundary: &Vec<Point>) -> impl Iterator<Item = Edge> {
    let mut edge_starts = boundary.iter().peekable();
    let p0 = std::iter::once(*edge_starts.peek().expect("Points"));
    let edge_ends = boundary.iter().skip(1).chain(p0);
    let edges = edge_starts.into_iter().zip(edge_ends).map(|(p1, p2)| Edge::from((p1, p2)));
    edges
}

pub fn get_boundary_edge_pairs(boundary: &Vec<Point>) -> impl Iterator<Item = (Edge, Edge)> {
    let mut edges_a = get_boundary_edges(boundary).peekable();
    let e0 = std::iter::once(edges_a.peek().expect("Edges").clone());
    let edges_b = get_boundary_edges(boundary).skip(1).chain(e0);
    let edges = edges_a.into_iter().zip(edges_b);
    edges
}


#[cfg(test)]
mod tests {
    use nalgebra::{point, distance};

    use crate::{geo::{contour::Contour, E}, poly};

    use super::*;

    fn check_winding(boundary: &Vec<Point>) -> Result<()> {
        let mut turned_left = false;
        let mut turned_right = false;

        for (e1, e2) in get_boundary_edge_pairs(boundary) {
            match e1.turning(e2.end()) {
                Turning::Left => turned_left = true,
                Turning::Right => turned_right = true,
                Turning::Collinear => {},
            }
        }

        if turned_right && !turned_left {
            bail!("Boundary winding is backwards.");
        } else if turned_right {
            bail!("Boundary is not convex.");
        }

        Ok(())
    }

    #[test]
    fn line_winding() -> Result<()> {
        let mut line = PathBuilder::new()
            .do_moveto(point![0.0, 0.0])?
            .do_lineto(point![0.0, 1.0])?
            .into_line(1.0)?;

        line.set_resolution(Some(5.0));

        check_winding(&line.boundary())?;

        Ok(())
    }

    #[test]
    fn circle_winding() -> Result<()> {
        let circle = Circle::new(point![0.0, 0.0], 5.0);

        check_winding(&circle.boundary())?;

        Ok(())
    }

    #[test]
    fn polygon_edges() -> Result<()> {
        let a = poly!(
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        );

        assert!(get_boundary_edges(&a.boundary).count() == 4);
        assert!(get_boundary_edge_pairs(&a.boundary).count() == 4);

        Ok(())
    }

    #[test]
    fn line_points() -> Result<()> {
        let mut line = PathBuilder::new()
            .do_moveto(point![0.0, 0.0])?
            .do_lineto(point![0.0, 1.0])?
            .into_line(1.0)?;

        line.set_resolution(Some(5.0));

        let contour: Contour = Contour::new(Box::new(line))?;
        let points: Vec<_> = contour.points().collect();

        assert!(points.len() == 4);

        let d3 = distance(&points[0], &point![-0.5, 0.0]);
        let d0 = distance(&points[1], &point![0.5, 0.0]);
        let d1 = distance(&points[2], &point![0.5, 1.0]);
        let d2 = distance(&points[3], &point![-0.5, 1.0]);

        assert!(d0 < E, "d0 {d0} is not close to 0");
        assert!(d1 < E, "d1 {d1} is not close to 0");
        assert!(d2 < E, "d2 {d2} is not close to 0");
        assert!(d3 < E, "d3 {d3} is not close to 0");

        Ok(())
    }

    #[test]
    fn line_contains() -> Result<()> {

        Ok(())
    }
}
