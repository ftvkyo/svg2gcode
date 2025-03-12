use std::iter::once;

use geo::{Coord, Intersects, IsConvex, Line, LineString, Polygon, Winding};

use super::{LineExt, Shape, EPSILON};


#[derive(Clone, Debug)]
pub struct ThickPolygon {
    inner: LineString,
    offset: f64,
}

impl ThickPolygon {
    pub fn new(mut boundary: LineString) -> Self {
        assert!(boundary.0.len() >= 3);
        assert!(boundary.is_closed());

        boundary.make_ccw_winding();
        assert!(boundary.is_ccw_convex(), "Polygon boundary is not convex");

        Self {
            inner: boundary,
            offset: 0.0,
        }
    }
}

impl Shape for ThickPolygon {
    fn offset(&mut self, offset: f64) {
        self.offset += offset;
    }
}

impl Into<Polygon> for ThickPolygon {
    fn into(self) -> Polygon {
        if self.offset < EPSILON {
            return Polygon::new(self.inner, vec![]);
        }

        let add_edge = |boundary: &mut Vec<Coord>, a: Coord, b: Coord, c: Coord| {
            let line1 = Line::new(a, b).shift_right(self.offset);
            let line2 = Line::new(b, c).shift_right(self.offset);

            assert!(!line1.intersects(&line2), "Stumbled upon a self-intersection between {line1:?} & {line2:?}");

            let arc = line1.find_arc(&line2, b);
            boundary.extend(arc);
        };

        let mut boundary = vec![];

        let p_last = self.inner.0[self.inner.0.len() - 2]; // The `-1` point is == the `0` point
        let p0 = self.inner.0[0];
        let p1 = self.inner.0[1];
        let last_window = [p_last, p0, p1];

        let mut windows = self.inner.0.windows(3).chain(once(&last_window as &[_]));
        while let Some([a, b, c]) = windows.next() {
            add_edge(&mut boundary, *a, *b, *c);
        }

        Polygon::new(LineString::new(boundary), vec![])
    }
}
