use anyhow::{ensure, Context, Result};
use svg::node::element::{Path as SvgPath, path::Data as SvgPathData};

use crate::geo::edge::{Edge, Turning};

use super::{Float, Point};


pub struct Contour {
    /// A closed loop of points, ordered counter-clockwise
    boundary: Vec<Point>,
}

impl Contour {
    pub fn new(boundary: Vec<Point>) -> Result<Self> {
        ensure!(boundary.len() >= 3);

        let mut s = Self { boundary };

        let mut turned_left = false;
        let mut turned_right = false;

        for (e1, e2) in s.edge_pairs()? {
            match e1.turning(&e2.end) {
                Turning::Left => turned_left = true,
                Turning::Right => turned_right = true,
                Turning::Collinear => {},
            }
        }

        if turned_right && !turned_left {
            eprintln!("Contour winding is backwards. Please fix.");
            s.boundary.reverse();
        }

        Ok(s)
    }

    pub fn points(&self) -> Result<impl DoubleEndedIterator<Item = &Point>> {
        Ok(self.boundary.iter())
    }

    pub fn edges(&self) -> Result<impl Iterator<Item = Edge>> {
        let mut edge_starts = self.points()?.peekable();
        let p0 = std::iter::once(*edge_starts.peek().context("No points?")?);
        let edge_ends = self.points()?.skip(1).chain(p0);
        let edges = edge_starts.into_iter().zip(edge_ends).map(|(p1, p2)| Edge::new(p1.clone(), p2.clone()));
        Ok(edges)
    }

    pub fn edge_pairs(&self) -> Result<impl Iterator<Item = (Edge, Edge)>> {
        let mut edges_a = self.edges()?.peekable();
        let e0 = std::iter::once(edges_a.peek().context("No edges?")?.clone());
        let edges_b = self.edges()?.skip(1).chain(e0);
        let edges = edges_a.into_iter().zip(edges_b);
        Ok(edges)
    }

    pub fn grow(&mut self, offset: Float) -> Result<()> {
        if offset == 0.0 {
            return Ok(());
        }

        ensure!(offset > 0.0);

        // TODO: delete edges when things become self-intersecting
        // TODO: rounded links?

        let mut edges = self.edges()?.map(|e| e.translate_right(offset)).peekable();
        let mut edge_prev = edges.peek().context("No edges?")?.clone();
        let edges = edges.skip(1).chain(std::iter::once(edge_prev.clone()));

        let mut boundary = vec![];

        for edge in edges {
            boundary.push(edge_prev.link(&edge)?);
            edge_prev = edge;
        }

        self.boundary = boundary;

        Ok(())
    }

    pub fn svg(&self) -> Result<SvgPath> {
        let mut points = self.points()?.peekable();
        let first = points.peek().context("Expected at least 1 point")?;

        let mut data = SvgPathData::new()
            .move_to((first.x, first.y));

        for p in points.skip(1) {
            data = data.line_to((p.x, p.y));
        }
        data = data.close();

        let path = SvgPath::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke");

        Ok(path)
    }

    pub fn is_convex(&self) -> Result<bool> {
        // Convex if never turning right

        let mut turned_right = false;

        for (e1, e2) in self.edge_pairs()? {
            match e1.turning(&e2.end) {
                Turning::Right => turned_right = true,
                _ => {},
            }
        }

        return Ok(!turned_right)
    }
}
