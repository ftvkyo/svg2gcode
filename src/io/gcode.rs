use std::iter::once;

use geo::{Coord, MultiPolygon};

use crate::{config::SharedFabConfig, fab::{FabData, Hole}, io::gcode_generator::GCodeGenerator};


fn make_gcode_contours(config: &SharedFabConfig, polygons: &MultiPolygon, depths: &Vec<f64>, feed: f64, rpm: f64) -> String {
    let mut gcode = GCodeGenerator::new(feed, rpm, config.safe_height);

    gcode.spindle_start_cwise();

    for poly in polygons {
        for contour in poly.interiors().into_iter().chain(once(poly.exterior())) {

            for &depth in depths {
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
    }

    gcode.spindle_stop();

    gcode.into_string()
}


fn make_gcode_drilling(config: &SharedFabConfig, holes: &Vec<Hole>, depth: f64, feed: f64, rpm: f64) -> String {
    let mut gcode = GCodeGenerator::new(feed, rpm, config.safe_height);

    gcode.spindle_start_cwise();

    for hole in holes {
        gcode.rapid(hole.center.x, hole.center.y);
        gcode.engage();
        gcode.move_z(-depth);
        gcode.disengage();
    }

    gcode.spindle_stop();

    gcode.into_string()
}


fn make_gcode_boring(config: &SharedFabConfig, holes: &Vec<Hole>, depth: f64, bit_radius: f64, feed: f64, rpm: f64) -> String {
    let mut gcode = GCodeGenerator::new(feed, rpm, config.safe_height);

    gcode.spindle_start_ccwise();

    for hole in holes {
        let offset = Coord {
            x: hole.radius - bit_radius,
            y: 0.0,
        };

        let helix = hole.center + offset;

        let turns = (depth / 0.25) as usize;

        gcode.rapid(helix.x, helix.y);
        gcode.engage();
        gcode.helix_ccwise(helix.x, helix.y, -depth, -offset.x, -offset.y, turns);
        gcode.arc_ccwise(helix.x, helix.y, -offset.x, -offset.y);
        gcode.disengage();
    }

    gcode.spindle_stop();

    gcode.into_string()
}


pub fn make_gcode(config: &SharedFabConfig, fd: &FabData) -> String {
    match fd {
        FabData::Contours { contours, depths, feed, rpm } => make_gcode_contours(config, contours, depths, *feed, *rpm),
        FabData::Drilling { holes, depth, feed, rpm } => make_gcode_drilling(config, holes, *depth, *feed, *rpm),
        FabData::Boring { holes, depth, bit_radius, feed, rpm } => make_gcode_boring(config, holes, *depth, *bit_radius, *feed, *rpm),
    }
}
