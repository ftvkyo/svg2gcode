use std::slice::Windows;

use geo::{Coord, Intersects, Line, LineString, Polygon, Vector2DOps};

use super::{LineExt, Shape, EPSILON};


#[derive(Clone, Debug)]
pub struct ThickLineString {
    inner: LineString,
    thickness: f64,
}

impl ThickLineString {
    pub fn new(line: LineString, thickness: f64) -> Self {
        assert!(line.0.len() >= 2);
        assert!(thickness > 0.0);
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
    }
}

impl Shape for ThickLineString {
    fn offset(&mut self, offset: f64) {
        self.thickness += offset * 2.0;
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

                if line1.intersects(&line2) {
                    let int = line1.find_intersection(&line2).expect("An intersection point");
                    boundary.push(int);
                } else {
                    let arc = line1.find_arc(&line2, *b);
                    boundary.extend(arc);
                }
            }
        };

        let mut boundary: Vec<Coord> = vec![];

        add_cap(&mut boundary, &inner.0);
        add_side(&mut boundary, inner.0.windows(3));

        inner.0.reverse();
        add_cap(&mut boundary, &inner.0);
        add_side(&mut boundary, inner.0.windows(3));

        // TODO: make sure there are no self-intersections

        Polygon::new(LineString::new(boundary), vec![])
    }
}
