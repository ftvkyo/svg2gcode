use std::slice::Windows;

use geo::{line_intersection::line_intersection, Centroid, Coord, Euclidean, Length, Line, LineIntersection, LineString, Polygon, Vector2DOps};
use log::{debug, warn};

use super::{IntoPolygon, LineExt, EPSILON};


#[derive(Clone, Debug)]
pub struct ThickLineString {
    inner: LineString,
    thickness: f64,
}

impl ThickLineString {
    pub fn new(line: LineString, thickness: f64) -> Self {
        assert!(line.0.len() >= 2);
        assert!(thickness > 0.0);
        assert!(line.length::<Euclidean>() > 0.0);

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

impl IntoPolygon for ThickLineString {
    fn into_polygon(self, resolution: f64) -> Polygon {
        let Self {
            mut inner,
            thickness,
        } = self;

        if inner.is_closed() {
            warn!("Found a closed loop of lines, ignoring thickness and interpreting as a polygon.");
            return Polygon::new(inner, vec![]);
        }

        let offset = thickness / 2.0;

        let add_cap = |boundary: &mut Vec<Coord>, points: &[Coord]| {
            let p0 = points[0];
            let p1 = points[1];

            let line_first = Line::new(p0, p1).shift_right(offset);
            let line_last = Line::new(p1, p0).shift_right(offset);

            debug!("Adding cap between {line_first:?} and {line_last:?}");

            boundary.extend(line_last.find_arc(&line_first, p0, resolution));
        };

        let add_side = |boundary: &mut Vec<Coord>, mut w: Windows<'_, Coord>| {
            while let Some([a, b, c]) = w.next() {
                let line1 = Line::new(*a, *b).shift_right(offset);
                let line2 = Line::new(*b, *c).shift_right(offset);

                debug!("Adding connection between {line1:?} and {line2:?}");

                let int = line_intersection(line1, line2);
                match int {
                    Some(LineIntersection::SinglePoint { intersection, .. }) => boundary.push(intersection),
                    Some(LineIntersection::Collinear { intersection }) => boundary.push(intersection.centroid().0),
                    None => boundary.extend(line1.find_arc(&line2, *b, resolution)),
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


#[cfg(test)]
mod tests {
    use geo::line_string;

    use crate::tests::init_test_logger;

    use super::*;


#[test]
fn thick_line_vertices() {
    init_test_logger();

    let l1 = ThickLineString::new(line_string![
        (x: 0.0, y: 0.0),
        (x: 0.0, y: 1.0),
    ], 0.02);
    let p1: Polygon = l1.into_polygon(0.1);

    let coords: Vec<_> = p1.exterior().coords().collect();

    assert_eq!(coords.len(), 5, "Got {coords:?}");
    assert_eq!(p1.interiors().len(), 0);

    let lines: Vec<_> = p1.exterior().lines().collect();

    // Two vertical lines
    assert!(lines.contains(&Line { start: Coord { x: 0.01, y: 0.0 }, end: Coord { x: 0.01, y: 1.0 } }));
    assert!(lines.contains(&Line { start: Coord { x: -0.01, y: 1.0 }, end: Coord { x: -0.01, y: 0.0 } }));

    // Two horizontal lines that form the caps at this resolution
    assert!(lines.contains(&Line { start: Coord { x: 0.01, y: 1.0 }, end: Coord { x: -0.01, y: 1.0 } }));
    assert!(lines.contains(&Line { start: Coord { x: -0.01, y: 0.0 }, end: Coord { x: 0.01, y: 0.0 } }));
}
}
