use std::f32::consts::TAU;

use anyhow::{ensure, Result};
use nalgebra::{point, vector, Matrix3};

use crate::types::{Float, Point, Vector};

fn is_turning_left(va: &(Point, Point), vb: &(Point, Point)) -> bool {
    let va = va.1 - va.0;
    let vb = vb.1 - vb.0;
    let va90 = Vector::new(-va.y, va.x);
    va90.dot(&vb) > 0.0
}

// TODO: make this a trait
#[derive(Clone, Debug)]
pub struct Contour {
    /// Boundary of the contour, counter-clockwise
    boundary: Vec<Point>,
}

impl Contour {
    pub fn points(&self) -> impl DoubleEndedIterator<Item = Point> {
        self.boundary.iter().copied()
    }

    pub fn edges(&self) -> impl Iterator<Item = (Point, Point)> {
        let points_a = self.points();
        let points_b = self.points().skip(1).chain(self.points().take(1));
        points_a.zip(points_b)
    }

    pub fn is_convex(&self) -> bool {
        // Not conves if any of the edges turns "right"

        let edges_a = self.edges();
        let edges_b = self.edges().skip(1).chain(self.edges().take(1));

        for (ea, eb) in edges_a.zip(edges_b) {
            if !is_turning_left(&ea, &eb) {
                return false;
            }
        }

        true
    }

    pub fn confines(&self, p: Point) -> Result<bool> {
        // A contour confines a point if the point is "to the left" of every edge

        // FIXME: This only works correctly for convex objects!
        ensure!(self.is_convex());

        for (p1, p2) in self.edges() {
            let v_edge = p2 - p1;
            let v_inwards = Vector::new(- v_edge.y, v_edge.x);
            let v_point = p - p1;

            let cos = v_inwards.dot(&v_point);

            if cos < 0.0 {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl Into<svg::node::element::Path> for &Contour {
    fn into(self) -> svg::node::element::Path {
        let first = self.points().next().unwrap();

        let mut data = svg::node::element::path::Data::new()
            .move_to((first.x, first.y));

        for p in self.points().skip(1) {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        let path = svg::node::element::Path::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke");

        path
    }
}

pub struct ContourUnclosed {
    pub(self) inner: Contour,
}

impl ContourUnclosed {
    pub fn points(&self) -> impl DoubleEndedIterator<Item = Point> {
        self.inner.boundary.iter().copied()
    }

    fn edges(&self) -> impl Iterator<Item = (Point, Point)> {
        let points_a = self.points();
        let points_b = self.points();
        points_a.zip(points_b.skip(1))
    }

    pub fn edges_reverse(&self) -> impl Iterator<Item = (Point, Point)> {
        let points_a = self.points().rev();
        let points_b = self.points().rev();
        points_b.zip(points_a.skip(1))
    }

    fn inflate_simple(self, thickness: Float) -> Result<Contour> {
        let mut contour = self.inner;

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

    pub fn expand(mut self, thickness: Float) -> Result<Contour> {
        if self.inner.boundary.len() == 2 {
            return self.inflate_simple(thickness);
        }

        ensure!(self.inner.boundary.len() > 2);

        // Hopefully the traces are not too crazy and we can resolve rectangle intersections one by one.

        let offset_line = |(p1, p2): (Point, Point)| {
            let line = p2 - p1;
            let v270 = Vector::new(line.y, - line.x).normalize() * thickness / 2.0;
            (p1 + v270, p2 + v270)
        };

        let get_angle = |line_a: (Point, Point), line_b: (Point, Point)| {
            let line_a = line_a.1 - line_a.0;
            let line_b = line_b.1 - line_b.0;
            line_a.angle(&line_b)
        };

        let mut boundary = vec![];

        let mut do_side = |edges: Vec<(Point, Point)>| {
            let last = edges.last().unwrap().clone();

            let mut edges = edges.into_iter();
            let mut prev = edges.next().unwrap();

            boundary.push(offset_line(prev).0);

            for curr in edges {
                let angle = get_angle(prev, curr);
                let turning_left = is_turning_left(&prev, &curr);

                let line = offset_line(curr);
                let change = thickness * (angle / 2.0).tan() / 2.0;

                // Line from 0 to 1
                let line_v = line.1 - line.0;

                let line_v = if turning_left {
                    - line_v.normalize() * change
                } else {
                    line_v.normalize() * change
                };

                boundary.push(line.0 + line_v);

                prev = curr;
            }

            boundary.push(offset_line(last).1);
        };

        do_side(self.edges().collect());
        do_side(self.edges_reverse().collect());

        self.inner.boundary = boundary;

        Ok(self.inner)
    }
}

#[must_use]
pub enum ContourFinalisation {
    Contour(Contour),
    Unclosed(ContourUnclosed),
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

        Ok(ContourFinalisation::Unclosed(ContourUnclosed { inner: self.inner }))
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::point;

    use super::*;

    #[test]
    fn contour_convex() {
        let contour = ContourBuilder::new_circle(point![0.0, 0.0], 1.0, 36);
        assert!(contour.is_convex());

        let mut contour = ContourBuilder::new_empty();
        contour.do_move(point![0.0, 1.0]).unwrap();
        contour.do_line(point![-1.0, -1.0]).unwrap();

        if let ContourFinalisation::Unclosed(contour) = contour.build().unwrap() {
            let contour = contour.expand(0.1).unwrap();
            assert!(contour.is_convex());
        } else {
            panic!("Contour was deflated");
        }

        let mut contour = ContourBuilder::new_empty();
        contour.do_move(point![0.0, 1.0]).unwrap();
        contour.do_line(point![-1.0, -1.0]).unwrap();
        contour.do_line(point![1.0, -1.0]).unwrap();
        contour.do_line(point![1.0, -2.0]).unwrap();
        contour.do_line(point![5.0, -2.0]).unwrap();
        contour.do_line(point![5.0, 0.0]).unwrap();
        contour.do_close().unwrap();

        if let ContourFinalisation::Contour(contour) = contour.build().unwrap() {
            assert!(!contour.is_convex());
        } else {
            panic!("Contour was deflated");
        }
    }

    #[test]
    fn contour_basic_confines() {
        let mut contour = ContourBuilder::new_empty();
        contour.do_move(point![0.0, 1.0]).unwrap();
        contour.do_line(point![-1.0, -1.0]).unwrap();
        contour.do_line(point![1.0, -1.0]).unwrap();
        contour.do_close().unwrap();

        if let ContourFinalisation::Contour(contour) = contour.build().unwrap() {
            assert!(contour.confines(point![0.0, 0.0]).unwrap());
            assert!(contour.confines(point![0.5, 0.0]).unwrap());
            assert!(contour.confines(point![-0.5, 0.0]).unwrap());
            assert!(contour.confines(point![0.0, 0.5]).unwrap());

            assert!(!contour.confines(point![2.0, 0.0]).unwrap());
            assert!(!contour.confines(point![0.0, 2.0]).unwrap());
            assert!(!contour.confines(point![-2.0, 0.0]).unwrap());
            assert!(!contour.confines(point![0.0, -2.0]).unwrap());
        } else {
            panic!("Contour was deflated");
        }
    }

    #[test]
    fn contour_circle_confines() {
        let contour = ContourBuilder::new_circle(point![0.0, 0.0], 1.0, 36);

        assert!(contour.confines(point![0.0, 0.0]).unwrap());
        assert!(contour.confines(point![0.69, 0.69]).unwrap());

        assert!(!contour.confines(point![1.0, 1.0]).unwrap());

        let contour = ContourBuilder::new_circle(point![1.0, 1.0], 1.0, 36);

        assert!(contour.confines(point![1.0, 1.0]).unwrap());
        assert!(contour.confines(point![1.69, 1.69]).unwrap());

        assert!(!contour.confines(point![0.0, 0.0]).unwrap());
    }
}
