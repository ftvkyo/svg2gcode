use nalgebra as na;

pub mod contour;
pub mod edge;
pub mod shape;


pub type Float = f32;
pub use std::f32::consts::TAU;
pub use std::f32::consts::PI;
pub const E: Float = 0.0001;

pub type Vector = na::Vector2<Float>;
pub type Point = na::Point2<Float>;


#[macro_export]
macro_rules! feq {
    ($f1:expr, $f2:expr) => {
        ($f1 - $f2).abs() < crate::geo::E
    }
}


#[macro_export]
macro_rules! p2eq {
    ($p1:ident, $p2:ident) => {
        crate::feq!($p1.x, $p2.x) && crate::feq!($p1.y, $p2.y)
    }
}
