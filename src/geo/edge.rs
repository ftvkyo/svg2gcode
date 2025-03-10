use nalgebra as na;

use crate::{feq, geo::E, p2eq};

use super::{Float, Point, Vector};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum Turning {
    Left,
    Collinear,
    Right,
}

#[derive(Clone, Debug)]
pub struct Edge {
    inner: (Point, Point),
}

impl std::fmt::Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = 8;
        let precision = 4;

        write!(f, "edge!({:width$.precision$}, {:width$.precision$}, {:width$.precision$}, {:width$.precision$})",
            self.inner.0.x,
            self.inner.0.y,
            self.inner.1.x,
            self.inner.1.y,
        )
    }
}

impl From<(Point, Point)> for Edge {
    fn from(value: (Point, Point)) -> Self {
        assert!(!p2eq!(value.0, value.1), "Tried to create a 0-length edge: from {} to {}", value.0, value.1);
        Self {
            inner: (value.0, value.1),
        }
    }
}

impl<'p> From<(&'p Point, &'p Point)> for Edge {
    fn from(value: (&'p Point, &'p Point)) -> Self {
        assert!(!p2eq!(value.0, value.1), "Tried to create a 0-length edge: from {} to {}", value.0, value.1);
        Self {
            inner: (*value.0, *value.1),
        }
    }
}


impl Edge {
    pub fn left(&self) -> Vector {
        let v_self = self.inner.1 - self.inner.0;
        na::vector![-v_self.y, v_self.x]
    }

    pub fn right(&self) -> Vector {
        let v_self = self.inner.1 - self.inner.0;
        na::vector![v_self.y, -v_self.x]
    }

    pub fn start(&self) -> &Point {
        &self.inner.0
    }

    pub fn end(&self) -> &Point {
        &self.inner.1
    }

    pub fn reverse(&self) -> Edge {
        Edge::from((self.end(), self.start()))
    }

    pub fn angle(&self, other: &Self) -> Float {
        let v1 = self.inner.1 - self.inner.0;
        let v2 = other.inner.1 - other.inner.0;
        v1.angle(&v2)
    }

    pub fn translate_right(&self, distance: Float) -> Self {
        // Direction of translation
        let v_self_270 = self.right().normalize();

        let start = self.inner.0 + v_self_270 * distance;
        let end = self.inner.1 + v_self_270 * distance;

        Self {
            inner: (start, end),
        }
    }

    /// Determine whether the edge sequence would "turn" left or right if `next` was the next point
    pub fn turning(&self, next: &Point) -> Turning {
        // Direction of "left"
        let v_self_90 = self.left().normalize();
        let v_next = next - self.start();
        let dot = v_self_90.dot(&v_next);

        if dot.abs() < E {
            Turning::Collinear
        } else if dot > 0.0 {
            Turning::Left
        } else {
            Turning::Right
        }
    }

    pub fn length(&self) -> Float {
        return (self.start() - self.end()).magnitude()
    }

    pub fn distance(&self, p: &Point) -> Float {
        let v = self.end() - self.start();

        let p2s = (self.start() - p).magnitude();
        let p2e = (self.end() - p).magnitude();

        if p2s < E || p2e < E {
            return 0.0;
        }

        if feq!(v.x, 0.0) {
            // `self` is vertical

            let py_between_se = self.start().y <= p.y && p.y <= self.end().y;
            let py_between_es = self.end().y <= p.y && p.y <= self.start().y;

            if py_between_se || py_between_es {
                // The closest point lies on the segment
                return (self.start().x - p.x).abs()
            }

            // The closest point is one of the segment's ends
            return p2s.min(p2e);
        }

        let turning1 = Self::from((*self.start(), self.start() + self.left())).turning(&p);
        let turning2 = Self::from((*self.end(), self.end() + self.left())).turning(&p);

        if turning1 > Turning::Left && turning2 < Turning::Right {
            // The closest point lies on the segment.
            // Find the intersection point using line intersection logic.

            // This goes through `p` and is perpendicular to `self`
            let fake_p_line = Edge::from((*p, p + self.left()));
            let intersection = self.find_intersection(&fake_p_line);

            return (intersection - p).magnitude();
        }

        // The closest point is one of the segment's ends
        return p2s.min(p2e);
    }

    /// Check if two edges intersect (and are not just touching)
    pub fn crosses(&self, other: &Self) -> bool {
        use Turning::*;

        // Two edges intersect if for each edge, both of its points are on different sides of the other edge.
        let t1 = self.turning(other.start());
        let t2 = self.turning(other.end());
        let t3 = other.turning(self.start());
        let t4 = other.turning(self.end());

        if t1 == Collinear || t2 == Collinear || t3 == Collinear || t4 == Collinear {
            // May be touching
            return false;
        }

        t1 != t2 && t3 != t4
    }

    pub fn touches(&self, other: &Self) -> bool {
        use Turning::*;

        // In general, two edges are touching if at least one point is collinear with the other edge, and points of each edge are not on the same side of the other edge
        let t1 = self.turning(other.start());
        let t2 = self.turning(other.end());
        let t3 = other.turning(self.start());
        let t4 = other.turning(self.end());

        if t1 == Collinear && t2 == Collinear && t3 == Collinear && t4 == Collinear {
            // Special case: the edges are collinear.
            return self.distance(other.start()) < E || self.distance(other.end()) < E;
        }

        let collinear = t1 == Collinear || t2 == Collinear || t3 == Collinear || t4 == Collinear;
        let side1 = t1 != t2;
        let side2 = t3 != t4;

        collinear && side1 && side2
    }

    /// Finds a point where lines defined by `self` and `other` intersect
    pub fn find_intersection(&self, other: &Self) -> Point {
        let self_dx = self.inner.1.x - self.inner.0.x;
        let self_dy = self.inner.1.y - self.inner.0.y;

        let other_dx = other.inner.1.x - other.inner.0.x;
        let other_dy = other.inner.1.y - other.inner.0.y;

        let closest_center = || {
            let self_closest = if other.distance(self.start()) < other.distance(self.end()) { self.start() } else { self.end() };
            let other_closest = if self.distance(other.start()) < self.distance(other.end()) { other.start() } else { other.end() };
            na::center(self_closest, other_closest)
        };

        if self_dy.abs() < E && other_dy.abs() < E {
            // Both lines are horizontal
            assert!((self.inner.0.y - other.inner.0.y).abs() < E, "Got two non-collinear horizontal edges");
            return closest_center();
        }

        if self_dx.abs() < E && other_dx.abs() < E {
            // Both lines are vertical
            assert!((self.inner.0.x - other.inner.0.x).abs() < E, "Got two non-collinear vertical edges");
            return closest_center();
        }

        if self_dx.abs() < E {
            // only `self` is vertical
            let x = self.inner.0.x;
            let y = (x - other.inner.0.x) * other_dy / other_dx + other.inner.0.y;
            return na::point![x, y];
        }

        if other_dx.abs() < E {
            // only `other` is vertical
            let x = other.inner.0.x;
            let y = (x - self.inner.0.x) * self_dy / self_dx + self.inner.0.y;
            return na::point![x, y];
        }

        let self_m = self_dy / self_dx;
        let other_m = other_dy / other_dx;

        if feq!(self_m, other_m) {
            // The lines are parallel, compare their value at x == 0
            let self_y = self_m * (- self.inner.0.x) + self.inner.0.y;
            let other_y = other_m * (- other.inner.0.x) + other.inner.0.y;
            assert!(feq!(self_y, other_y), "Got two parallel but not collinear edges");
            return closest_center();
        }

        let x = (self.inner.0.x * self_m - other.inner.0.x * other_m - self.inner.0.y + other.inner.0.y) / (self_m - other_m);
        let y = self_m * (x - self.inner.0.x) + self.inner.0.y;

        return na::point![x, y];
    }

    /// Finds a series of points to smoothly connect the end of `self` to the start of `other`
    pub fn find_arc(&self, other: &Self, radius: Float, resolution: Float) -> Vec<Point> {
        assert!(radius > 0.0);
        assert!(resolution > 0.0);

        // 1. Determine whether the arc is clockwise or counterclockwise

        let turn_angle = self.angle(other);
        let arc_angle = match self.turning(other.start()) {
            Turning::Left => turn_angle,
            Turning::Collinear => panic!("Tried to make a smooth link for connected edges: {self}, {other}"),
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

        points
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::point;

    use crate::edge;

    use super::*;
    use Turning::*;

    #[test]
    fn turning() {
        let v = Edge::from((point![0.0, 0.0], point![0.0, 1.0]));

        assert_eq!(v.turning(&point![-1.0, -1.0]), Left);
        assert_eq!(v.turning(&point![-1.0, 0.0]), Left);
        assert_eq!(v.turning(&point![-1.0, 1.0]), Left);
        assert_eq!(v.turning(&point![-1.0, 2.0]), Left);

        assert_eq!(v.turning(&point![0.0, -1.0]), Collinear);
        assert_eq!(v.turning(&point![0.0, 0.0]), Collinear);
        assert_eq!(v.turning(&point![0.0, 1.0]), Collinear);
        assert_eq!(v.turning(&point![0.0, 2.0]), Collinear);

        assert_eq!(v.turning(&point![1.0, -1.0]), Right);
        assert_eq!(v.turning(&point![1.0, 0.0]), Right);
        assert_eq!(v.turning(&point![1.0, 1.0]), Right);
        assert_eq!(v.turning(&point![1.0, 2.0]), Right);
    }

    #[test]
    fn crossing_and_touching() {
        {
            let e1 = edge!(0.0, 0.0, 1.0, 1.0);
            let e2 = edge!(0.0, 1.0, 1.0, 0.0);
            assert!(e1.crosses(&e2));
            assert!(!e1.touches(&e2));

            let e1 = edge!(0.0, 0.0, 1.0, 1.0);
            let e2 = edge!(0.0, 1.0, 1.0, 2.0);
            assert!(!e1.crosses(&e2));
            assert!(!e1.touches(&e2));
        }

        let v = edge!(0.0, -1.0, 0.0, 1.0);
        let h = edge!(-1.0, 0.0, 1.0, 0.0);

        assert!(!v.crosses(&v));
        assert!(v.touches(&v));
        assert!(!h.crosses(&h));
        assert!(h.touches(&h));

        assert!(v.crosses(&h));
        assert!(!v.touches(&h));
    }

    #[test]
    fn distance() {
        let e = Edge::from((point![0.0, 0.0], point![1.0, 1.0]));

        assert!(feq!(e.distance(&point![0.0, 0.0]), 0.0));
        assert!(feq!(e.distance(&point![1.0, 1.0]), 0.0));
        assert!(feq!(e.distance(&point![0.5, 0.5]), 0.0));

        assert!(feq!(e.distance(&point![1.0, 0.0]), 2.0f32.sqrt() / 2.0));
        assert!(feq!(e.distance(&point![0.0, 1.0]), 2.0f32.sqrt() / 2.0));

        assert!(feq!(e.distance(&point![-1.0, 0.0]), 1.0));
        assert!(feq!(e.distance(&point![0.0, -1.0]), 1.0));
        assert!(feq!(e.distance(&point![1.0, 2.0]), 1.0));
        assert!(feq!(e.distance(&point![2.0, 1.0]), 1.0));

        assert!(feq!(e.distance(&point![-1.0, -1.0]), 2.0f32.sqrt()));
        assert!(feq!(e.distance(&point![2.0, 2.0]), 2.0f32.sqrt()));
    }

    #[test]
    fn intersection() {
        {
            let e1 = edge!(0.0, 0.0, 0.0, 1.0);
            let e2 = edge!(0.0, 1.0, 1.0, 1.0);
            assert!(p2eq!(e1.find_intersection(&e2), point![0.0, 1.0]));
        }

        {
            let e1 = edge!( 1.0,  2.0,  1.0,  1.0);
            let e2 = edge!( 1.0,  0.0,  1.0,  1.0);
            let e3 = edge!( 1.0,  1.0,  0.0,  1.0);

            assert!(p2eq!(e1.find_intersection(&e2), point![1.0, 1.0]));
            assert!(p2eq!(e1.find_intersection(&e3), point![1.0, 1.0]));
            assert!(p2eq!(e2.find_intersection(&e3), point![1.0, 1.0]));
        }
    }
}
