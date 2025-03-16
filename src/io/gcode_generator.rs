
#[derive(Debug, PartialEq)]
enum GCodeState {
    Stopped,
    SpinningDisengaged,
    SpinningEngaged,
}

use GCodeState::*;

pub struct GCodeGenerator {
    safe_height: f64,

    state: GCodeState,
    actions: Vec<String>,
}

impl GCodeGenerator {
    pub fn new(feed: f64, rpm: f64, safe_height: f64) -> Self {
        Self {
            safe_height,
            state: GCodeState::Stopped,
            actions: vec![
                format!("G90"), // Absolute
                format!("F{feed}"), // Feed
                format!("S{rpm}"), // Spindle
                format!("G0 Z{safe_height}"), // Go to safe height
            ],
        }
    }

    pub fn spindle_start_cwise(&mut self) {
        assert_eq!(self.state, Stopped);
        self.actions.push(format!("M3"));
        self.state = SpinningDisengaged;
    }

    pub fn spindle_start_ccwise(&mut self) {
        assert_eq!(self.state, Stopped);
        self.actions.push(format!("M4"));
        self.state = SpinningDisengaged;
    }

    pub fn spindle_stop(&mut self) {
        assert_eq!(self.state, SpinningDisengaged);
        self.actions.push(format!("M5"));
        self.state = Stopped;
    }

    pub fn engage(&mut self) {
        assert_eq!(self.state, SpinningDisengaged);
        self.actions.push(format!("G1 Z0"));
        self.state = SpinningEngaged;
    }

    pub fn disengage(&mut self) {
        assert_eq!(self.state, SpinningEngaged);
        self.actions.push(format!("G1 Z{}", self.safe_height));
        self.state = SpinningDisengaged;
    }

    pub fn rapid(&mut self, x: f64, y: f64) {
        assert_ne!(self.state, SpinningEngaged);
        self.actions.push(format!("G0 X{x} Y{y}"));
    }

    pub fn move_xy(&mut self, x: f64, y: f64) {
        assert_eq!(self.state, SpinningEngaged);
        self.actions.push(format!("G1 X{x} Y{y}"));
    }

    pub fn move_z(&mut self, z: f64) {
        assert_eq!(self.state, SpinningEngaged);
        self.actions.push(format!("G1 Z{z}"));
    }

    pub fn helix_ccwise(&mut self, end_x: f64, end_y: f64, end_z: f64, offset_x: f64, offset_y: f64, turns: usize) {
        assert_eq!(self.state, SpinningEngaged);

        // 1. Select the axis
        // - G17 - Z-axis, XY-plane
        // - G18 - Y-axis, XZ-plane
        // - G19 - X-axis, YZ-plane

        self.actions.push(format!("G17"));

        // 2. Do the move
        // As viewed from the positive end of the axis:
        // - G2 - clockwise
        // - G3 - counterclockwise

        self.actions.push(format!("G3 X{end_x} Y{end_y} Z{end_z} I{offset_x} J{offset_y} P{turns}"));
    }

    pub fn arc_ccwise(&mut self, end_x: f64, end_y: f64, offset_x: f64, offset_y: f64) {
        self.actions.push(format!("G17"));
        self.actions.push(format!("G3 X{end_x} Y{end_y} I{offset_x} J{offset_y}"));
    }

    pub fn into_string(mut self) -> String {
        assert_eq!(self.state, Stopped);
        self.actions.push(format!("M2"));
        self.actions.join("\n")
    }
}
