use std::borrow::Cow;

use anyhow::{bail, ensure, Result};
use nalgebra as na;

use crate::{feq, geo::{E, PI, TAU}};

use super::{Float, Point, Vector};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Turning {
    Left,
    Collinear,
    Right,
}

#[derive(Clone, Debug)]
pub struct Edge<'p> {
    inner: (Cow<'p, Point>, Cow<'p, Point>),
}

impl From<(Point, Point)> for Edge<'_> {
    fn from(value: (Point, Point)) -> Self {
        Self {
            inner: (Cow::Owned(value.0), Cow::Owned(value.1)),
        }
    }
}

impl<'p> From<(&'p Point, &'p Point)> for Edge<'p> {
    fn from(value: (&'p Point, &'p Point)) -> Self {
        Self {
            inner: (Cow::Borrowed(value.0), Cow::Borrowed(value.1)),
        }
    }
}

impl Edge<'_> {
    pub fn left(&self) -> Vector {
        let v_self = *self.inner.1 - *self.inner.0;
        na::vector![-v_self.y, v_self.x]
    }

    pub fn right(&self) -> Vector {
        let v_self = *self.inner.1 - *self.inner.0;
        na::vector![v_self.y, -v_self.x]
    }

    pub fn start(&self) -> &'_ Point {
        self.inner.0.as_ref()
    }

    pub fn end(&self) -> &'_ Point {
        self.inner.1.as_ref()
    }

    pub fn angle(&self, other: &Self) -> Float {
        let v1 = *self.inner.1 - *self.inner.0;
        let v2 = *other.inner.1 - *other.inner.0;
        v1.angle(&v2)
    }

    pub fn translate_right(&self, distance: Float) -> Edge<'static> {
        // Direction of translation
        let v_self_270 = self.right().normalize();

        let start = *self.inner.0 + v_self_270 * distance;
        let end = *self.inner.1 + v_self_270 * distance;

        Edge {
            inner: (Cow::Owned(start), Cow::Owned(end)),
        }
    }

    /// Determine whether the edge sequence would "turn" left or right if `next` was the next point
    pub fn turning(&self, next: &Point) -> Turning {
        // Direction of "left"
        let v_self_90 = self.left().normalize();
        let v_next = next - *self.inner.1;
        let dot = v_self_90.dot(&v_next);

        if dot.abs() < E {
            Turning::Collinear
        } else if dot > 0.0 {
            Turning::Left
        } else {
            Turning::Right
        }
    }

    /// Check if two edges intersect or at least touch each other
    pub fn intersects(&self, other: &Self) -> bool {
        // Two edges intersect if for both edges, both of its points are on different sides of the other edge.
        self.turning(other.start()) != self.turning(other.end())
        && other.turning(self.start()) != other.turning(self.end())
    }

    /// Finds a point where lines defined by `self` and `other` intersect
    pub fn find_intersection(&self, other: &Self) -> Result<Point> {
        let self_dx = self.inner.1.x - self.inner.0.x;
        let self_dy = self.inner.1.y - self.inner.0.y;

        let other_dx = other.inner.1.x - other.inner.0.x;
        let other_dy = other.inner.1.y - other.inner.0.y;

        if self_dy.abs() < E && other_dy.abs() < E {
            // Both lines are horizontal
            ensure!((self.inner.0.y - other.inner.0.y).abs() < E, "Got two non-collinear horizontal edges");
            return Ok(na::center(&self.inner.1, &other.inner.0));
        }

        if self_dx.abs() < E && other_dx.abs() < E {
            // Both lines are vertical
            ensure!((self.inner.0.x - other.inner.0.x).abs() < E, "Got two non-collinear vertical edges");
            return Ok(na::center(&self.inner.1, &other.inner.0));
        }

        if self_dx.abs() < E {
            // only `self` is vertical
            let x = self.inner.0.x;
            let y = (x - other.inner.0.x) * other_dy / other_dx + other.inner.0.y;
            return Ok(na::point![x, y]);
        }

        if other_dx.abs() < E {
            // only `other` is vertical
            let x = other.inner.0.x;
            let y = (x - self.inner.0.x) * self_dy / self_dx + self.inner.0.y;
            return Ok(na::point![x, y]);
        }

        let self_m = self_dy / self_dx;
        let other_m = other_dy / other_dx;

        if feq!(self_m, other_m) {
            // The lines are parallel, compare their value at x == 0
            let self_y = self_m * (- self.inner.0.x) + self.inner.0.y;
            let other_y = other_m * (- other.inner.0.x) + other.inner.0.y;
            ensure!(feq!(self_y, other_y), "Got two parallel but not collinear edges");
        }

        let x = (self.inner.0.x * self_m - other.inner.0.x * other_m - self.inner.0.y + other.inner.0.y) / (self_m - other_m);
        let y = self_m * (x - self.inner.0.x) + self.inner.0.y;

        return Ok(na::point![x, y]);
    }

    /// Finds a series of points to smoothly connect the end of `self` to the start of `other`
    pub fn find_arc(&self, other: &Self, radius: Float, resolution: Float) -> Result<Vec<Point>> {
        ensure!(radius > 0.0);
        ensure!(resolution > 0.0);

        // 1. Determine whether the arc is clockwise or counterclockwise

        let turn_angle = self.angle(other);
        let arc_angle = match self.turning(other.start()) {
            Turning::Left => turn_angle,
            Turning::Collinear => bail!("Tried to make a smooth link for connected edges"),
            Turning::Right => - turn_angle,
        };

        // 2. Determine arc length

        let arc_length = radius * arc_angle.abs();
        let arc_segments = (arc_length / resolution).ceil() as usize;
        let arc_rot = na::Rotation2::new(arc_angle / arc_segments as Float);

        // 3. Calculate the points

        let origin = self.end() + self.left().normalize() * radius;

        let mut points = Vec::with_capacity(arc_segments + 1);

        let mut v_rot = self.right().normalize() * radius;
        for _ in 0..=arc_segments {
            points.push(origin + v_rot);
            v_rot = arc_rot * v_rot;
        }

        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::point;

    use super::*;
    use Turning::*;

    #[test]
    fn turning() -> Result<()> {
        let v = Edge::from((point![0.0, 0.0], point![0.0, 1.0]));

        ensure!(v.turning(&point![-1.0, -1.0]) == Left);
        ensure!(v.turning(&point![0.0, 2.0]) == Collinear);
        ensure!(v.turning(&point![1.0, 1.0]) == Right);

        Ok(())
    }

    #[test]
    fn intersecting() {
        let e1 = Edge::from((point![0.0, 0.0], point![1.0, 1.0]));
        let e2 = Edge::from((point![0.0, 1.0], point![1.0, 0.0]));

        assert!(e1.intersects(&e2));

        let e1 = Edge::from((point![0.0, 0.0], point![1.0, 1.0]));
        let e2 = Edge::from((point![0.0, 1.0], point![1.0, 2.0]));

        assert!(!e1.intersects(&e2));
    }

    #[test]
    fn linking() -> Result<()> {
        let e1 = Edge::from((point![0.0, 0.0], point![0.0, 1.0]));
        let e2 = Edge::from((point![0.0, 1.0], point![1.0, 1.0]));

        let link = e1.find_intersection(&e2)?;

        ensure!(link == point![0.0, 1.0]);

        Ok(())
    }

    #[test]
    fn linking_special() -> Result<()> {
        // Two vertical unconnected edges
        let e1 = Edge::from((point![0.0, 0.0], point![0.0, 1.0]));
        let e2 = Edge::from((point![1.0, 0.0], point![1.0, 1.0]));
        let link = e1.find_intersection(&e2);
        ensure!(link.is_err(), "{e1:?} and {e2:?} linked to {link:?}");

        // Two horizontal unconnected edges
        let e1 = Edge::from((point![0.0, 0.0], point![1.0, 0.0]));
        let e2 = Edge::from((point![0.0, 1.0], point![1.0, 1.0]));
        let link = e1.find_intersection(&e2);
        ensure!(link.is_err(), "{e1:?} and {e2:?} linked to {link:?}");

        // Two collinear unconnected edges
        let e1 = Edge::from((point![0.0, 0.0], point![1.0, 1.0]));
        let e2 = Edge::from((point![0.0, 1.0], point![1.0, 2.0]));
        let link = e1.find_intersection(&e2);
        ensure!(link.is_err(), "{e1:?} and {e2:?} linked to {link:?}");

        Ok(())
    }
}
