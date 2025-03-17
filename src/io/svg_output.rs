use geo::{LineString, Point};
use svg::{node::element, Document};

use crate::fab::{FabContourData, FabData, FabHoleData, FabOperation, Hole};

pub struct ViewBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl ViewBox {
    pub fn new() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        }
    }

    pub fn include(&mut self, (x, y): (f64, f64)) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    pub fn add_margin(&mut self, margin: f64) {
        self.min_x -= margin;
        self.min_y -= margin;
        self.max_x += margin;
        self.max_y += margin;
    }

    pub fn get(&self) -> (f64, f64, f64, f64) {
        (self.min_x, self.min_y, self.max_x - self.min_x, self.max_y - self.min_y)
    }
}

fn make_svg_path(mut points: impl Iterator<Item = Point>, view_box: &mut ViewBox) -> element::Path {
    let p0 = points.next().unwrap();

    let mut data = element::path::Data::new();
    data = data.move_to(p0.x_y());
    view_box.include(p0.x_y());

    for p in points {
        data = data.line_to(p.x_y());
        view_box.include(p.x_y());
    }

    data = data.close();

    element::Path::new()
        .set("d", data)
        .set("vector-effect", "non-scaling-stroke")
}

fn make_svg_paths(contours: &Vec<LineString>, fill: &str, stroke: &str, view_box: &mut ViewBox) -> element::Group {
    let mut g_contours = element::Group::new()
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1);

    for contour in contours {
        g_contours = g_contours.add(make_svg_path(contour.points().skip(1), view_box));
    }

    g_contours
}

fn make_svg_holes(holes: &Vec<Hole>, fill: &str, view_box: &mut ViewBox) -> element::Group {
    let mut g_holes = element::Group::new()
        .set("fill", fill)
        .set("stroke", "none");

    for hole in holes {
        view_box.include((hole.center.x, hole.center.y));

        g_holes = g_holes.add(element::Circle::new()
            .set("cx", hole.center.x)
            .set("cy", hole.center.y)
            .set("r", hole.radius));
    }

    g_holes
}

pub fn make_svg(fds: &Vec<FabData>) -> Document {
    let mut view_box = ViewBox::new();

    let mut doc = Document::new();

    for data in fds {
        let g = match &data.operation {
            FabOperation::Engrave(FabContourData { contours, .. }) => make_svg_paths(contours, "none", "#4774AAFF", &mut view_box),
            FabOperation::Cut(FabContourData { contours, .. }) => make_svg_paths(contours, "#4774AA22", "red", &mut view_box),
            FabOperation::Drilling(FabHoleData { holes, .. }) => make_svg_holes(holes, "#329544FF", &mut view_box),
            FabOperation::Boring { data: FabHoleData { holes, .. }, .. } => make_svg_holes(holes, "#893566FF", &mut view_box),
        };

        doc = doc.add(g);
    }

    view_box.add_margin(5.0);

    doc.set("viewBox", view_box.get())
}
