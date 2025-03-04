use crate::types::{Point, Vector};

pub struct Contour {
    /// Boundary of the contour, counter-clockwise
    pub(self) boundary: Vec<Point>,
}

impl Contour {
    pub fn edges(&self) -> impl Iterator<Item = (Point, Point)> {
        let points_a = self.boundary.iter().copied();
        let points_b = self.boundary.iter().skip(1).chain(self.boundary.iter().take(1)).copied();
        points_a.zip(points_b)
    }

    pub fn confines(&self, p: Point) -> bool {
        // A contour confines a point if the point is "to the left" of every edge

        for (p1, p2) in self.edges() {
            let v_edge = p2 - p1;
            let v_inwards = Vector::new(- v_edge.y, v_edge.x);
            let v_point = p - p1;

            let cos = v_inwards.dot(&v_point);

            if cos < 0.0 {
                return false;
            }
        }

        true
    }
}


pub struct ContourBuilder {
    contour: Contour,
}

impl ContourBuilder {
    pub fn new() -> Self {
        Self {
            contour: Contour {
                boundary: Vec::new(),
            },
        }
    }

    pub fn point(mut self, p: Point) -> Self {
        self.contour.boundary.push(p);
        self
    }

    pub fn build(self) -> Contour {
        self.contour
    }
}


#[cfg(test)]
mod tests {
    use nalgebra::point;

    use super::*;

    #[test]
    fn contour_confines() {
        let contour = ContourBuilder::new()
            .point(point![0.0, 1.0])
            .point(point![-1.0, -1.0])
            .point(point![1.0, -1.0])
            .build();

        assert!(contour.confines(point![0.0, 0.0]));
        assert!(contour.confines(point![0.5, 0.0]));
        assert!(contour.confines(point![-0.5, 0.0]));
        assert!(contour.confines(point![0.0, 0.5]));

        assert!(!contour.confines(point![2.0, 0.0]));
        assert!(!contour.confines(point![0.0, 2.0]));
        assert!(!contour.confines(point![-2.0, 0.0]));
        assert!(!contour.confines(point![0.0, -2.0]));
    }
}
