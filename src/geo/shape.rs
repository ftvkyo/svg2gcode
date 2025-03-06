use anyhow::{ensure, Context, Result};
use nalgebra as na;

use crate::geo::{contour::Contour, edge::Edge};

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
    pub fn into_contour(self, cap_steps: usize) -> Result<Contour> {
        ensure!(cap_steps > 0);
        let cap_rot = na::Rotation2::new(PI / cap_steps as f32);

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
        for _ in 0..=cap_steps {
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
        for _ in 0..=cap_steps {
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

    pub fn into_contour(self, sides: usize) -> Result<Contour> {
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


#[cfg(test)]
mod tests {
    use nalgebra::{point, distance};

    use crate::geo::E;

    use super::*;

    #[test]
    fn line_points() {
        let line = PathBuilder::new()
            .do_moveto(point![0.0, 0.0]).unwrap()
            .do_lineto(point![0.0, 1.0]).unwrap()
            .into_line(1.0).unwrap();

        let contour: Contour = line.into_contour(1).unwrap();
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
