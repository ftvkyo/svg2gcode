use nalgebra as na;

pub mod contour;
pub mod edge;


pub type Float = f32;
pub use std::f32::consts::TAU;
pub use std::f32::consts::PI;
pub const E: Float = 0.0001;

pub type Vector = na::Vector2<Float>;
pub type Point = na::Point2<Float>;
