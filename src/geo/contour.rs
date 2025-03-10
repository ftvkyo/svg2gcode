use std::iter::once;

use log::{debug, error};
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use crate::p2eq;

use super::{debug::fmt_edges, edge::Edge, shape::{Shape, ShapeE}, Point};


pub struct Contour {
    /// A closed loop of points, ordered counter-clockwise
    boundary: Vec<Point>,
    /// Constituents
    area: Vec<ShapeE>,
}

impl Contour {
    pub fn new(shape: ShapeE) -> Self {
        Self {
            boundary: shape.boundary(),
            area: vec![shape],
        }
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

    fn break_edges(&self, other: &Self) -> Vec<Edge> {
        let mut this_broken = vec![];

        for tseg in self.edges() {
            debug!("  Breaking   {tseg}");

            let mut ints: Vec<_> = other.edges()
                .filter_map(|oseg| {
                    let crosses = tseg.crosses(&oseg);
                    let touches = tseg.touches(&oseg);

                    if crosses {
                        debug!("    Crosses  {oseg}");
                    }
                    if touches {
                        debug!("    Touches  {oseg}");
                    }

                    assert!(!crosses || !touches, "Edges should not cross and touch at the same time");

                    if crosses || touches {
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

            let mut prev = *tseg.start();
            for end in ints.into_iter().chain(once(*tseg.end())) {
                if p2eq!(prev, end) {
                    debug!("    Skipping {end} - zero-length");
                    continue;
                }

                let e = Edge::from((prev, end));
                if other.contains(&prev) && other.contains(&end) {
                    debug!("    Skipping {e} - inside of the other contour");
                    prev = end;
                    continue;
                }

                debug!("    Saving   {e}");
                this_broken.push(e);
                prev = end;
            }
        }

        this_broken
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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BelongsTo {
    A,
    B,
}

impl BelongsTo {
    pub fn other(&self) -> Self {
        use BelongsTo::*;
        match self {
            A => B,
            B => A,
        }
    }
}

pub struct Contours {
    pub contours: Vec<Contour>,
    pub problems: Vec<Edge>,
}


impl<I: Iterator<Item = Contour>> From<I> for Contours {
    fn from(value: I) -> Self {
        Self {
            contours: value.collect(),
            problems: vec![],
        }
    }
}

impl Contours {
    fn merge(&mut self, a: Contour, b: Contour) {
        debug!("Starting merging");
        debug!("Contour A:\n{}", fmt_edges(a.edges()));
        debug!("Contour B:\n{}", fmt_edges(b.edges()));

        debug!("Breaking edges of A...");
        let edges_a = a.break_edges(&b);
        debug!("Edges A:\n{}", fmt_edges(edges_a.iter().cloned()));

        debug!("Breaking edges of B...");
        let edges_b = b.break_edges(&a);
        debug!("Edges B:\n{}", fmt_edges(edges_b.iter().cloned()));


        // Mark edges as belonging to one or the other shape
        let edges_a = edges_a.into_iter().map(|s| (s, BelongsTo::A));
        let edges_b = edges_b.into_iter().map(|s| (s, BelongsTo::B));

        debug!("Matching the edges into a boundary...");

        let find_next = |edges: &Vec<(Edge, BelongsTo)>, start: &Point, contour: BelongsTo| -> Option<(usize, Edge, BelongsTo)> {
            edges.iter()
                .enumerate()
                .find_map(move |(i, (e, c))| if *c == contour && p2eq!(start, e.start()) {
                    Some((i, e.clone(), *c))
                } else {
                    None
                })
        };

        let mut edges: Vec<_> = edges_a.chain(edges_b).collect();
        let area: Vec<_> = a.area.into_iter().chain(b.area.into_iter()).collect();

        // Find loops
        while !edges.is_empty() {
            let mut edges_acc = vec![];

            let (mut prev_edge, mut prev_cont) = edges.pop().unwrap();
            debug!("  Starting with {prev_edge}, belongs to {prev_cont:?}");
            edges_acc.push(prev_edge.clone());

            loop {
                if p2eq!(prev_edge.end(), edges_acc[0].start()) {
                    debug!("  Finished a loop");
                    break;
                }

                let matching = find_next(&edges, prev_edge.end(), prev_cont.other())
                    .or_else(|| find_next(&edges, prev_edge.end(), prev_cont));

                if let Some((next_i, next_edge, next_cont)) = matching {
                    debug!("  Found {next_edge}, belongs to {next_cont:?}");
                    edges_acc.push(next_edge);
                    (prev_edge, prev_cont) = edges.remove(next_i);
                } else {
                    error!(
                        "Could not find the next edge after {prev_edge}. Remaining:\n{}",
                        fmt_edges(edges.iter().map(|(e, _)| e).cloned())
                    );

                    self.problems.extend(edges_acc);
                    self.problems.extend(edges.into_iter().map(|(e, _)| e));
                    return;
                }
            }

            self.contours.push(Contour {
                boundary: edges_acc.iter().map(|e| *e.start()).collect(),
                area: area.clone(),
            });
        }
    }

    pub fn merge_all(&mut self) -> () {
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
                // `j` is always > `i` so it's safe to remove `j` and then `i`
                let b = self.contours.remove(j);
                let a = self.contours.remove(i);
                self.merge(a, b);
                continue;
            }

            break;
        }
    }
}


#[cfg(test)]
mod tests {
    use anyhow::Result;
    use nalgebra::point;

    use crate::geo::{contour::Contours, debug::init_test_logger, shape::{PathBuilder, Shape, ShapeE}};

    use super::Contour;

    macro_rules! cont {
        ($($tt:tt),* $(,)?) => {
            $crate::geo::contour::Contour::new($crate::geo::shape::ShapeE::Poly($crate::poly!($($tt),*)))
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

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 8);
        assert_eq!(cs.problems.len(), 0);

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

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 8);
        assert_eq!(cs.problems.len(), 0);

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

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 8);
        assert_eq!(cs.problems.len(), 0);

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

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 9);
        assert_eq!(cs.problems.len(), 0);

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

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 8);
        assert_eq!(cs.problems.len(), 0);

        Ok(())
    }

    #[test]
    fn merging_touching_edge_edge_corners() -> Result<()> {
        init_test_logger();

        // +---+
        // |   +-+
        // |   | |
        // +---+-+

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 7);
        assert_eq!(cs.problems.len(), 0);

        Ok(())
    }

    #[test]
    fn merging_touching_edge_edge_corner_corner() -> Result<()> {
        init_test_logger();

        // +-+-+
        // | | |
        // +-+-+

        let a = cont!(
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

        let mut cs = Contours::from([a, b].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.contours[0].boundary.len(), 6);
        assert_eq!(cs.problems.len(), 0);

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

        let mut l1 = l1.into_line(1.0, 1.0)?;
        let mut l2 = l2.into_line(1.0, 1.0)?;

        l1.grow(5.0);
        l2.grow(5.0);

        let l1 = Contour::new(ShapeE::Line(l1));
        let l2 = Contour::new(ShapeE::Line(l2));

        assert!(l1.is_mergeable(&l2));

        assert!(!l2.contains(&point![64.7, 17.8]));
        assert!(!l2.contains(&point![63.7, 18.0]));

        let mut cs = Contours::from([l1, l2].into_iter());
        cs.merge_all();

        assert_eq!(cs.contours.len(), 1);
        assert_eq!(cs.problems.len(), 0);

        Ok(())
    }
}
