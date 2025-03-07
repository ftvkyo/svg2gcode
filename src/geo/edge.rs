use anyhow::{ensure, Result};
use nalgebra as na;

use crate::geo::E;

use super::{Float, Point, Vector};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Turning {
    Left,
    Collinear,
    Right,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub start: Point,
    pub end: Point,
}

impl Edge {
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
        }
    }

    pub fn left(&self) -> Vector {
        let v_self = self.end - self.start;
        na::vector![-v_self.y, v_self.x]
    }

    pub fn right(&self) -> Vector {
        let v_self = self.end - self.start;
        na::vector![v_self.y, -v_self.x]
    }

    /// Determine whether the edge sequence would "turn" left or right if `next` was the next point
    pub fn turning(&self, next: &Point) -> Turning {
        // Direction of "left"
        let v_self_90 = self.left().normalize();
        let v_next = next - self.end;
        let dot = v_self_90.dot(&v_next);

        if dot.abs() < E {
            Turning::Collinear
        } else if dot > 0.0 {
            Turning::Left
        } else {
            Turning::Right
        }
    }

    pub fn translate_right(&self, distance: Float) -> Self {
        // Direction of translation
        let v_self_270 = self.right().normalize();

        Self {
            start: self.start + v_self_270 * distance,
            end: self.end + v_self_270 * distance,
        }
    }

    /// Finds a point to link the second point of `self` with the first point of `other` (meant to replace them)
    pub fn link(&self, other: &Self) -> Result<Point> {
        let self_dx = self.end.x - self.start.x;
        let self_dy = self.end.y - self.start.y;

        let other_dx = other.end.x - other.start.x;
        let other_dy = other.end.y - other.start.y;

        // TODO: consider non-vertical & non-horizontal collinear cases

        if self_dy.abs() < E && other_dy.abs() < E {
            // Both lines are horizontal
            ensure!((self.start.y - other.start.y).abs() < E, "Got two non-collinear horizontal edges");
            return Ok(na::center(&self.end, &other.start));
        }

        if self_dx.abs() < E && other_dx.abs() < E {
            // Both lines are vertical
            ensure!((self.start.x - other.start.x).abs() < E, "Got two non-collinear vertical edges");
            return Ok(na::center(&self.end, &other.start));
        }

        if self_dx.abs() < E {
            // only `self` is vertical
            let x = self.start.x;
            let y = (x - other.start.x) * other_dy / other_dx + other.start.y;
            return Ok(na::point![x, y]);
        }

        if other_dx.abs() < E {
            // only `other` is vertical
            let x = other.start.x;
            let y = (x - self.start.x) * self_dy / self_dx + self.start.y;
            return Ok(na::point![x, y]);
        }

        let self_m = self_dy / self_dx;
        let other_m = other_dy / other_dx;

        let x = (self.start.x * self_m - other.start.x * other_m - self.start.y + other.start.y) / (self_m - other_m);
        let y = self_m * (x - self.start.x) + self.start.y;

        return Ok(na::point![x, y]);
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::point;

    use super::*;
    use Turning::*;

    #[test]
    fn turning() -> Result<()> {
        let v = Edge::new(point![0.0, 0.0], point![0.0, 1.0]);

        ensure!(v.turning(&point![-1.0, -1.0]) == Left);
        ensure!(v.turning(&point![0.0, 2.0]) == Collinear);
        ensure!(v.turning(&point![1.0, 1.0]) == Right);

        Ok(())
    }

    #[test]
    fn linking() -> Result<()> {
        let e1 = Edge::new(point![0.0, 0.0], point![0.0, 1.0]);
        let e2 = Edge::new(point![0.0, 1.0], point![1.0, 1.0]);

        let link = e1.link(&e2)?;

        ensure!(link == point![0.0, 1.0]);

        Ok(())
    }
}
