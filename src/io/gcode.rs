use geo::{Coord, LineString, Vector2DOps};

use crate::{config::SharedFabConfig, fab::{FabContourData, FabData, FabHoleData, FabOperation, Hole}, io::gcode_generator::GCodeGenerator};


fn find_next_contour(contours: &Vec<&LineString>, now: Coord) -> usize {
    let mut contours: Vec<_> = contours.iter().copied().enumerate().collect();

    contours.sort_by(|(_, a), (_, b)| (a.0[0] - now).magnitude_squared().total_cmp(&(b.0[0] - now).magnitude_squared()));

    return contours[0].0;
}


fn make_gcode_contours(config: &SharedFabConfig, data: &FabContourData, feed: f64, rpm: f64) -> String {
    let mut contours: Vec<_> = data.contours.iter().collect();

    let mut gcode = GCodeGenerator::new(feed, rpm, config.safe_height);

    gcode.spindle_start_cwise();

    let mut now = Coord { x: 0.0, y: 0.0 };
    while !contours.is_empty() {
        let contour = contours.remove(find_next_contour(&contours, now));
        now = contour.0[0];

        for &depth in &data.depths {
            let mut points = contour.coords();
            let p0 = points.next().unwrap();
            gcode.rapid(p0.x, p0.y);

            gcode.engage();
            gcode.move_z(-depth);

            while let Some(p) = points.next() {
                gcode.move_xy(p.x, p.y);
            }

            gcode.disengage();
        }
    }

    gcode.spindle_stop();

    gcode.into_string()
}


fn find_next_hole(holes: &Vec<&Hole>, now: Coord) -> usize {
    let mut holes: Vec<_> = holes.iter().copied().enumerate().collect();

    holes.sort_by(|(_, a), (_, b)| (a.center - now).magnitude_squared().total_cmp(&(b.center - now).magnitude_squared()));

    return holes[0].0;
}


fn make_gcode_drilling(config: &SharedFabConfig, data: &FabHoleData, feed: f64, rpm: f64) -> String {
    let mut holes: Vec<_> = data.holes.iter().collect();

    let mut gcode = GCodeGenerator::new(feed, rpm, config.safe_height);

    gcode.spindle_start_cwise();

    let mut now = Coord { x: 0.0, y: 0.0 };
    while !holes.is_empty() {
        let hole = holes.remove(find_next_hole(&holes, now));
        now = hole.center;

        gcode.rapid(hole.center.x, hole.center.y);
        gcode.engage();
        gcode.move_z(-data.depth);
        gcode.disengage();
    }

    gcode.spindle_stop();

    gcode.into_string()
}


fn make_gcode_boring(config: &SharedFabConfig, data: &FabHoleData, depth_per_turn: f64, bit_radius: f64, feed: f64, rpm: f64) -> String {
    let mut holes: Vec<_> = data.holes.iter().collect();

    let mut gcode = GCodeGenerator::new(feed, rpm, config.safe_height);

    gcode.spindle_start_ccwise();

    let mut now = Coord { x: 0.0, y: 0.0 };
    while !holes.is_empty() {
        let hole = holes.remove(find_next_hole(&holes, now));
        now = hole.center;

        let offset = Coord {
            x: hole.radius - bit_radius,
            y: 0.0,
        };

        let helix = hole.center + offset;

        let turns = (data.depth / depth_per_turn) as usize;

        gcode.rapid(helix.x, helix.y);
        gcode.engage();
        gcode.helix_ccwise(helix.x, helix.y, -data.depth, -offset.x, -offset.y, turns);
        gcode.arc_ccwise(helix.x, helix.y, -offset.x, -offset.y);
        gcode.disengage();
    }

    gcode.spindle_stop();

    gcode.into_string()
}


pub fn make_gcode(config: &SharedFabConfig, fd: &FabData) -> String {
    let feed = fd.feed;
    let rpm = fd.rpm;
    match &fd.operation {
        | FabOperation::Engrave(data)
        | FabOperation::Cut(data) => make_gcode_contours(config, data, feed, rpm),

        FabOperation::Drilling(data) => make_gcode_drilling(config, data, feed, rpm),

        FabOperation::Boring {
            data,
            depth_per_turn,
            bit_radius,
        } => make_gcode_boring(config, data, *depth_per_turn, *bit_radius, feed, rpm),
    }
}
