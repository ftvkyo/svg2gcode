use svg::{node::element, Document};

use crate::input::Primitives;

pub fn make_svg(shapes: Primitives) -> Document {
    let mut min_x: f64 = 0.0;
    let mut max_x: f64 = 0.0;
    let mut min_y: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    let mut g_contours = element::Group::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1);

    for polygon in shapes.polygons() {
        for p in polygon.exterior().points() {
            let x = p.x();
            let y = p.y();

            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        let mut points = polygon.exterior().points();

        let p0 = points.next().unwrap();

        let mut data = element::path::Data::new();
        data = data.move_to(p0.x_y());

        for p in points {
            data = data.line_to(p.x_y());
        }

        let path = element::Path::new()
            .set("d", data)
            .set("vector-effect", "non-scaling-stroke");

        g_contours = g_contours.add(path);
    }

    Document::new()
        .add(g_contours)
        .set("viewBox", (min_x - 5.0, min_y - 5.0, max_x - min_x + 10.0, max_y - min_y + 10.0))
}
