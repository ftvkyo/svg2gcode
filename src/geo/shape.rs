use anyhow::{ensure, Context, Result};
use nalgebra as na;

use crate::{feq, geo::{contour::Contour, edge::Edge}, p2eq};

use super::{Float, Point, PI, TAU};


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

    pub fn into_contour(self) -> Result<Contour> {
        ensure!(self.points.len() >= 3, "Can only close an area with at least 3 points");
        Contour::new(self.points)
    }

    pub fn into_line(self, thickness: Float) -> Result<Line> {
        ensure!(self.points.len() >= 2, "A line should have at least 2 points");
        ensure!(thickness > 0.0, "Thickness must be greater than 0");
        Ok(Line {
            points: self.points,
            thickness,
        })
    }
}


pub struct Line {
    points: Vec<Point>,
    thickness: Float,
}

impl Line {
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

    pub fn into_contour(self, cap_segments: usize) -> Result<Contour> {
        ensure!(cap_segments > 0);
        let cap_rot = na::Rotation2::new(PI / cap_segments as f32);

        let Line {
            points,
            thickness,
        } = self;

        let map_edge = |(p1, p2): (&Point, &Point)| Edge::new(p1.clone(), p2.clone()).translate_right(thickness / 2.0);

        let edge_first = Edge::new(points[0].clone(), points[1].clone());
        let edge_last = Edge::new(points[points.len() - 2].clone(), points[points.len() - 1].clone());

        let mut boundary = vec![];

        // Find start line cap

        let mut v_cap_start = edge_first.left().normalize() * thickness / 2.0;
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

        let mut v_cap_end = edge_last.right().normalize() * thickness / 2.0;
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

    pub fn into_contour(self, segments: usize) -> Result<Contour> {
        ensure!(segments > 0);
        ensure!(self.radius > 0.0);

        let rot = na::Matrix3::new_rotation(TAU / segments as Float);
        let center = self.center.to_homogeneous();

        let mut boundary = vec![];
        let mut v = na::vector![0.0, self.radius, 1.0];
        for _ in 0..segments {
            let p = center + v;
            boundary.push(na::point![p.x, p.y]);
            v = rot * v;
        }

        Contour::new(boundary)
    }
}


#[cfg(test)]
mod tests {
    use nalgebra::{point, distance};

    use crate::geo::E;

    use super::*;

    #[test]
    fn line_points() -> Result<()> {
        let line = PathBuilder::new()
            .do_moveto(point![0.0, 0.0])?
            .do_lineto(point![0.0, 1.0])?
            .into_line(1.0)?;

        let contour: Contour = line.into_contour(1)?;
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
