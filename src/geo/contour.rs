use std::iter::once;

use anyhow::Result;
use log::{debug, warn};
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use crate::p2eq;

use super::{debug::fmt_edges, edge::Edge, shape::Shape, Point};


pub struct Contour {
    /// A closed loop of points, ordered counter-clockwise
    boundary: Vec<Point>,
    /// Constituents
    area: Vec<Box<dyn Shape>>,
}

impl Contour {
    pub fn new(shape: Box<dyn Shape>) -> Result<Self> {
        Ok(Self {
            boundary: shape.boundary(),
            area: vec![shape],
        })
    }

    pub fn points(&self) -> impl DoubleEndedIterator<Item = &Point> {
        self.boundary.iter()
    }

    pub fn edges(&self) -> impl Iterator<Item = Edge> {
        let starts = self.boundary.iter();
        let ends = self.boundary.iter()
            .skip(1)
            .chain(once(&self.boundary[0]));
        starts.zip(ends).map(|(p1, p2)| Edge::from((p1, p2)))
    }

    pub fn contains(&self, p: &Point) -> bool {
        for shape in &self.area {
            if shape.contains(p) {
                return true;
            }
        }

        return false;
    }

    pub fn is_superset_of(&self, other: &Self) -> bool {
        for o_point in other.points() {
            if !self.contains(o_point) {
                // If there is a point of `other` that is not in `self`, then `other` is not contained within `self`
                return false;
            }
        }

        return true;
    }

    pub fn is_mergeable(&self, other: &Self) -> bool {
        for s_edge in self.edges() {
            for o_edge in other.edges() {
                if s_edge.crosses(&o_edge) || s_edge.touches(&o_edge) {
                    // If any of the edges cross or touch, contours are mergeable
                    return true;
                }
            }
        }

        return false;
    }

    pub fn merge(&mut self, other: Self) -> Result<()> {
        debug!("Starting merging");
        debug!("Contour A:\n{}", fmt_edges(self.edges()));
        debug!("Contour B:\n{}", fmt_edges(other.edges()));

        let break_edges = |this: &Self, other: &Self| -> Result<Vec<Edge>> {
            let mut this_broken = vec![];

            for tseg in this.edges() {
                debug!("Checking edge    {tseg}...");

                let mut ints: Vec<_> = other.edges()
                    .filter_map(|oseg| {
                        let crosses = tseg.crosses(&oseg);
                        let touches = tseg.touches(&oseg);

                        if crosses {
                            debug!("  Crosses {oseg}");
                        }
                        if touches {
                            debug!("  Touches {oseg}");
                        }

                        assert!(!crosses || !touches, "Edges should not cross and touch at the same time");

                        if tseg.crosses(&oseg) || tseg.touches(&oseg) {
                            Some(tseg.find_intersection(&oseg))
                        } else {
                            None
                        }
                    })
                    .collect();

                ints.sort_by(|aint, bint| {
                    (tseg.start() - aint).magnitude_squared()
                        .total_cmp(&(tseg.start() - bint).magnitude_squared())
                });

                debug!("  Intersections found: {}", ints.len());

                let mut prev = *tseg.start();
                for end in ints.into_iter().chain(once(*tseg.end())) {
                    if p2eq!(prev, end) {
                        warn!("    Skipping {prev} -> {end} as zero length");
                        continue;
                    }

                    let e = Edge::from((prev, end));
                    debug!("  Checking subedge {e}");

                    if other.contains(&prev) && other.contains(&end) {
                        debug!("    Skipping as inside of the other contour");
                        prev = end;
                        continue;
                    }

                    this_broken.push(e);
                    prev = end;
                }
            }

            Ok(this_broken)
        };

        debug!("Breaking edges of A...");
        let edges_self = break_edges(&self, &other)?;
        debug!("Edges A:\n{}", fmt_edges(edges_self.iter().cloned()));

        debug!("Breaking edges of B...");
        let edges_other = break_edges(&other, &self)?;
        debug!("Edges B:\n{}", fmt_edges(edges_other.iter().cloned()));

        /* === */

        #[derive(Debug, PartialEq, Eq)]
        enum BelongsTo {
            First,
            Second,
        }

        // Mark edges as belonging to one or the other shape
        let edges_self = edges_self.into_iter().map(|s| (s, BelongsTo::First));
        let edges_other = edges_other.into_iter().map(|s| (s, BelongsTo::Second));

        /* === */

        debug!("Matching the edges into a boundary...");

        let mut edges_all: Vec<_> = edges_self.chain(edges_other).collect();

        let (mut prev_e, mut prev_b) = edges_all.remove(0);
        debug!("  Starting with edge {prev_e}, belongs to {prev_b:?}");

        let mut boundary: Vec<Point> = vec![*prev_e.start()];
        while !edges_all.is_empty() {
            // First, try to find an edge that belongs the other contour
            let matching = edges_all.iter()
                .enumerate()
                .filter(|(_, (_, b))| *b != prev_b)
                .find(|(_, (e, _))| {
                    p2eq!(e.start(), prev_e.end())
                });

            // Otherwise, try to find an edge that belongs to the current contour
            let matching = matching.or_else(||
                edges_all.iter()
                    .enumerate()
                    .filter(|(_, (_, b))| *b == prev_b)
                    .find(|(_, (e, _))| {
                        p2eq!(e.start(), prev_e.end())
                    })
            );

            if let Some((next_i, (next_e, next_b))) = matching {
                debug!("  Found next segment {next_e}, belongs to {next_b:?}");
                boundary.push(*next_e.start());
                (prev_e, prev_b) = edges_all.remove(next_i);
            } else {
                warn!(
                    "Could not find the next edge after {prev_e}. Remaining:\n{}",
                    fmt_edges(edges_all.iter().map(|(e, _)| e).cloned())
                );
                break;
            }
        }

        self.boundary = boundary;
        self.area.extend(other.area.into_iter());

        Ok(())
    }

    pub fn svg(&self) -> SvgPath {
        let first = self.boundary[0];
        let mut data = SvgPathData::new()
            .move_to((first.x, first.y));

        for p in &self.boundary[1..] {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        SvgPath::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke")
    }
}


pub struct Repository {
    contours: Vec<Contour>
}

impl Repository {
    pub fn new() -> Self {
        Self {
            contours: vec![],
        }
    }

    pub fn add(&mut self, contour: Contour) -> Result<()> {
        for existing in self.contours.iter() {
            if existing.is_superset_of(&contour) {
                return Ok(());
            }
        }

        for existing in self.contours.iter_mut() {
            if existing.is_mergeable(&contour) {
                return existing.merge(contour);
            }
        }

        // Didn't merge with anything
        self.contours.push(contour);

        Ok(())
    }

    pub fn merge_all(&mut self) -> Result<()> {
        loop {
            let mut to_merge = None;

            for i in 0..self.contours.len() {
                for j in (i+1)..self.contours.len() {
                    if self.contours[i].is_mergeable(&self.contours[j]) {
                        to_merge = Some((i, j));
                        break;
                    }
                }
            }

            if let Some((i, j)) = to_merge {
                // `j` is always > `i` so it's safe to remove `j` and still use `i`
                let to_merge = self.contours.remove(j);
                self.contours[i].merge(to_merge)?;
                continue;
            }

            break;
        }

        Ok(())
    }

    pub fn contours(self) -> Vec<Contour> {
        self.contours
    }
}


#[cfg(test)]
mod tests {
    use anyhow::Result;
    use nalgebra::point;

    use crate::geo::{debug::init_test_logger, shape::{PathBuilder, Shape}};

    use super::Contour;

    macro_rules! cont {
        ($($tt:tt),* $(,)?) => {
            $crate::geo::contour::Contour::new(Box::new($crate::poly!($($tt),*))).unwrap()
        }
    }

    #[test]
    fn itself() -> Result<()> {
        init_test_logger();

        let a = cont!(
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        );

        assert!(a.is_mergeable(&a));
        assert!(a.is_superset_of(&a));

        Ok(())
    }

    #[test]
    fn unmergeable() -> Result<()> {
        init_test_logger();

        // +-+ +-+
        // | | | |
        // +-+ +-+

        let a = cont!(
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        );

        let b = cont!(
            [2.0, 0.0],
            [2.0, 1.0],
            [3.0, 1.0],
            [3.0, 0.0],
        );

        assert!(!a.is_mergeable(&b));
        assert!(!a.is_superset_of(&b));

        assert!(!b.is_mergeable(&a));
        assert!(!b.is_superset_of(&a));

        Ok(())
    }

    #[test]
    fn superset() -> Result<()> {
        init_test_logger();

        // +---+
        // |+-+|
        // || ||
        // |+-+|
        // +---+

        let a = cont!(
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 3.0],
            [0.0, 3.0],
        );

        let b = cont!(
            [1.0, 1.0],
            [2.0, 1.0],
            [2.0, 2.0],
            [1.0, 2.0],
        );

        assert!(!a.is_mergeable(&b));
        assert!(a.is_superset_of(&b));

        assert!(!b.is_mergeable(&a));
        assert!(!b.is_superset_of(&a));

        Ok(())
    }

    #[test]
    fn merging_basic() -> Result<()> {
        init_test_logger();

        // +---+
        // |   |
        // | +-+-+
        // | | | |
        // +-+-+ |
        //   |   |
        //   +---+

        let mut a = cont!(
            [0.0, 0.0],
            [0.0, 2.0],
            [2.0, 2.0],
            [2.0, 0.0],
        );

        let b = cont!(
            [1.0, 1.0],
            [1.0, 3.0],
            [3.0, 3.0],
            [3.0, 1.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 8);

        Ok(())
    }

    #[test]
    fn merging_diagonal() -> Result<()> {
        init_test_logger();

        //   ^ ^
        //  / X \
        // < < > >
        //  \ X /
        //   v v

        let mut a = cont!(
            [0.0, 1.0],
            [2.0, 2.0],
            [4.0, 1.0],
            [2.0, 0.0],
        );

        let b = cont!(
            [1.0, 1.0],
            [3.0, 2.0],
            [5.0, 1.0],
            [3.0, 0.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 8);

        Ok(())
    }

    #[test]
    fn merging_touching_corners() -> Result<()> {
        init_test_logger();

        // +-+
        // | |
        // +-+-+
        //   | |
        //   +-+

        let mut a = cont!(
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
        );

        let b = cont!(
            [1.0, 1.0],
            [1.0, 2.0],
            [2.0, 2.0],
            [2.0, 1.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 8);

        Ok(())
    }

    #[test]
    fn merging_touching_corner_edge() -> Result<()> {
        init_test_logger();

        // +---+  ^
        // |   | / \
        // |   |<   >
        // |   | \ /
        // +---+  v

        let mut a = cont!(
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 2.0],
            [0.0, 2.0],
        );

        let b = cont!(
            [3.0, 0.0],
            [4.0, 1.0],
            [3.0, 2.0],
            [2.0, 1.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 9);

        Ok(())
    }

    #[test]
    fn merging_touching_edge_edge() -> Result<()> {
        init_test_logger();

        // +---+
        // |   +-+
        // |   | |
        // |   +-+
        // +---+

        let mut a = cont!(
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 3.0],
            [0.0, 3.0],
        );

        let b = cont!(
            [1.0, 1.0],
            [2.0, 1.0],
            [2.0, 2.0],
            [1.0, 2.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 8);

        Ok(())
    }

    #[test]
    fn merging_touching_edge_edge_corners() -> Result<()> {
        init_test_logger();

        // +---+
        // |   +-+
        // |   | |
        // +---+-+

        let mut a = cont!(
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 2.0],
            [0.0, 2.0],
        );

        let b = cont!(
            [1.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 7);

        Ok(())
    }

    #[test]
    fn merging_touching_edge_edge_corner_corner() -> Result<()> {
        init_test_logger();

        // +-+-+
        // | | |
        // +-+-+

        let mut a = cont!(
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        );

        let b = cont!(
            [1.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
        );

        assert!(a.is_mergeable(&b));
        a.merge(b)?;

        assert_eq!(a.boundary.len(), 6);

        Ok(())
    }

    #[test]
    fn merging_lowres_lines() -> Result<()> {
        init_test_logger();

        let mut l1 = PathBuilder::new();
        l1.add_moveby([
            point![41.340783, 27.773344],
            point![7.023144, 0.07981],
            point![5.826018, -15.163607],
            point![8.539505, -0.07981],
        ].into_iter())?;

        let mut l2 = PathBuilder::new();
        l2.add_moveto([
            point![44.692738, 12.849162],
            point![59.776537, 27.134876],
        ].into_iter())?;

        let mut l1 = l1.into_line(1.0)?;
        let mut l2 = l2.into_line(1.0)?;

        l1.grow(5.0);
        l2.grow(5.0);

        let mut l1 = Contour::new(Box::new(l1))?;
        let l2 = Contour::new(Box::new(l2))?;

        assert!(l1.is_mergeable(&l2));

        assert!(!l2.contains(&point![64.7, 17.8]));
        assert!(!l2.contains(&point![63.7, 18.0]));

        let _ = l1.merge(l2)?;

        Ok(())
    }
}
