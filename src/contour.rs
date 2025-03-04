use std::f32::consts::TAU;

use anyhow::{ensure, Result};
use nalgebra::{point, vector, Matrix3};

use crate::types::{Float, Point, Vector};

#[derive(Clone, Debug)]
pub struct Contour {
    /// Boundary of the contour, counter-clockwise
    boundary: Vec<Point>,
}

impl Contour {
    pub fn points(&self) -> impl Iterator<Item = Point> {
        self.boundary.iter().copied()
    }

    pub fn edges(&self) -> impl Iterator<Item = (Point, Point)> {
        let points_a = self.boundary.iter().copied();
        let points_b = self.boundary.iter().skip(1).chain(self.boundary.iter().take(1)).copied();
        points_a.zip(points_b)
    }

    // FIXME: This only works correctly for convex objects!
    pub fn confines(&self, p: Point) -> bool {
        // A contour confines a point if the point is "to the left" of every edge

        for (p1, p2) in self.edges() {
            let v_edge = p2 - p1;
            let v_inwards = Vector::new(- v_edge.y, v_edge.x);
            let v_point = p - p1;

            let cos = v_inwards.dot(&v_point);

            if cos < 0.0 {
                return false;
            }
        }

        true
    }
}

pub struct ContourUnclosed {
    pub(self) inner: Contour,
}

impl ContourUnclosed {
    pub fn inflate(self, thickness: Float) -> Result<Contour> {
        let mut contour = self.inner;

        ensure!(contour.boundary.len() == 2);

        let line_p1 = contour.boundary[0];
        let line_p2 = contour.boundary[1];

        let line = line_p2 - line_p1;
        let v90 = Vector::new(- line.y, line.x).normalize() * thickness / 2.0;
        let v270 = - v90;

        let p1 = line_p1 + v270;
        let p2 = line_p2 + v270;
        let p3 = line_p2 + v90;
        let p4 = line_p1 + v90;

        contour.boundary = vec![
            p1,
            p2,
            p3,
            p4,
        ];

        Ok(contour)
    }
}

#[must_use]
pub enum ContourFinalisation {
    Contour(Contour),
    Deflated(ContourUnclosed),
}

pub struct ContourBuilder {
    inner: Contour,

    /// Whether the shape was closed
    closed: bool,
}

impl ContourBuilder {
    pub fn new_empty() -> Self {
        Self {
            inner: Contour {
                boundary: vec![],
            },
            closed: false,
        }
    }

    pub fn new_circle(center: Point, r: Float, sides: usize) -> Contour {
        let rot = Matrix3::new_rotation(TAU / sides as Float);

        let center = center.to_homogeneous();

        let mut points = vec![];
        let mut v = vector![0.0, r, 1.0];
        for _ in 0..sides {
            let p = center + v;
            points.push(point![p.x, p.y]);
            v = rot * v;
        }

        Contour {
            boundary: points,
        }
    }

    pub fn do_move(&mut self, p: Point) -> Result<()> {
        ensure!(!self.closed);
        self.inner.boundary.push(p);
        Ok(())
    }

    pub fn do_line(&mut self, p: Point) -> Result<()> {
        ensure!(!self.closed);
        ensure!(self.inner.boundary.len() > 0, "Line is not supported as the first command");
        self.inner.boundary.push(p);
        Ok(())
    }

    pub fn do_close(&mut self) -> Result<()> {
        ensure!(!self.closed);
        ensure!(self.inner.boundary.len() >= 3, "A boundary needs to be at least 3 points to allow closing");
        self.closed = true;
        Ok(())
    }

    pub fn build(self) -> Result<ContourFinalisation> {
        if self.closed {
            return Ok(ContourFinalisation::Contour(self.inner));
        }

        Ok(ContourFinalisation::Deflated(ContourUnclosed { inner: self.inner }))
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::point;

    use super::*;

    #[test]
    fn contour_basic_confines() {
        let mut contour = ContourBuilder::new_empty();
        contour.do_move(point![0.0, 1.0]).unwrap();
        contour.do_line(point![-1.0, -1.0]).unwrap();
        contour.do_line(point![1.0, -1.0]).unwrap();
        contour.do_close().unwrap();

        if let ContourFinalisation::Contour(contour) = contour.build().unwrap() {
            assert!(contour.confines(point![0.0, 0.0]));
            assert!(contour.confines(point![0.5, 0.0]));
            assert!(contour.confines(point![-0.5, 0.0]));
            assert!(contour.confines(point![0.0, 0.5]));

            assert!(!contour.confines(point![2.0, 0.0]));
            assert!(!contour.confines(point![0.0, 2.0]));
            assert!(!contour.confines(point![-2.0, 0.0]));
            assert!(!contour.confines(point![0.0, -2.0]));
        } else {
            panic!("Contour was deflated");
        }
    }

    #[test]
    fn contour_circle_confines() {
        let contour = ContourBuilder::new_circle(point![0.0, 0.0], 1.0, 36);

        assert!(contour.confines(point![0.0, 0.0]));
        assert!(contour.confines(point![0.69, 0.69]));

        assert!(!contour.confines(point![1.0, 1.0]));

        let contour = ContourBuilder::new_circle(point![1.0, 1.0], 1.0, 36);

        assert!(contour.confines(point![1.0, 1.0]));
        assert!(contour.confines(point![1.69, 1.69]));

        assert!(!contour.confines(point![0.0, 0.0]));
    }
}
