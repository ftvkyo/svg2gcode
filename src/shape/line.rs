use std::slice::Windows;

use geo::{line_intersection::line_intersection, Centroid, Coord, Euclidean, Length, Line, LineIntersection, LineString, Polygon, Simplify, Vector2DOps};

use crate::shape::SIMPLIFY_RESOLUTION;

use super::{LineExt, EPSILON};


#[derive(Clone, Debug)]
pub struct ThickLineString {
    inner: LineString,
    thickness: f64,
}

impl ThickLineString {
    pub fn new(mut line: LineString, thickness: f64) -> Self {
        assert!(line.0.len() >= 2);
        assert!(thickness > 0.0);
        assert!(line.length::<Euclidean>() > 0.0);

        line = line.simplify(&SIMPLIFY_RESOLUTION);

        Self {
            inner: line,
            thickness,
        }
    }

    pub fn can_join(&self, other: &Self) -> bool {
        let a = self;
        let b = other;

        if (a.thickness - b.thickness).abs() > EPSILON {
            return false;
        }

        let a1 = *a.inner.0.first().unwrap();
        let a2 = *a.inner.0.last().unwrap();

        let b1 = *b.inner.0.first().unwrap();
        let b2 = *b.inner.0.last().unwrap();

        return (a1 - b1).magnitude_squared() < EPSILON
            || (a1 - b2).magnitude_squared() < EPSILON
            || (a2 - b1).magnitude_squared() < EPSILON
            || (a2 - b2).magnitude_squared() < EPSILON;
    }

    pub fn join(&mut self, other: Self) {
        let a = self;
        let b = other;

        let a1 = *a.inner.0.first().unwrap();
        let a2 = *a.inner.0.last().unwrap();

        let b1 = *b.inner.0.first().unwrap();
        let b2 = *b.inner.0.last().unwrap();

        if (a1 - b1).magnitude_squared() < EPSILON {
            a.inner.0.reverse();
            a.inner.0.extend(b.inner.0.into_iter().skip(1));
        } else if (a1 - b2).magnitude_squared() < EPSILON {
            a.inner.0.reverse();
            a.inner.0.extend(b.inner.0.into_iter().rev().skip(1));
        } else if (a2 - b1).magnitude_squared() < EPSILON {
            a.inner.0.extend(b.inner.0.into_iter().skip(1));
        } else if (a2 - b2).magnitude_squared() < EPSILON {
            a.inner.0.extend(b.inner.0.into_iter().rev().skip(1));
        } else {
            panic!("Tried to merge lines that are not connected");
        }

        a.inner = a.inner.simplify(&SIMPLIFY_RESOLUTION);
    }
}

impl Into<Polygon> for ThickLineString {
    fn into(self) -> Polygon {
        let Self {
            mut inner,
            thickness,
        } = self;

        let offset = thickness / 2.0;

        let add_cap = |boundary: &mut Vec<Coord>, points: &[Coord]| {
            let p0 = points[0];
            let p1 = points[1];

            let line_first = Line::new(p0, p1).shift_right(offset);
            let line_last = Line::new(p1, p0).shift_right(offset);

            boundary.extend(line_last.find_arc(&line_first, p0));
        };

        let add_side = |boundary: &mut Vec<Coord>, mut w: Windows<'_, Coord>| {
            while let Some([a, b, c]) = w.next() {
                let line1 = Line::new(*a, *b).shift_right(offset);
                let line2 = Line::new(*b, *c).shift_right(offset);

                let int = line_intersection(line1, line2);
                match int {
                    Some(LineIntersection::SinglePoint { intersection, .. }) => boundary.push(intersection),
                    Some(LineIntersection::Collinear { intersection }) => boundary.push(intersection.centroid().0),
                    None => boundary.extend(line1.find_arc(&line2, *b)),
                }
            }
        };

        let mut boundary: Vec<Coord> = vec![];

        add_cap(&mut boundary, &inner.0);
        add_side(&mut boundary, inner.0.windows(3));

        inner.0.reverse();
        add_cap(&mut boundary, &inner.0);
        add_side(&mut boundary, inner.0.windows(3));

        Polygon::new(LineString::new(boundary), vec![])
    }
}
