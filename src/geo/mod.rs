use nalgebra as na;

pub use std::f32::consts::TAU;
pub use std::f32::consts::PI;

pub mod contour;
pub mod debug;
pub mod edge;
pub mod shape;


pub type Float = f32;
pub const E: Float = 0.001;

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
    ($p1:expr, $p2:expr) => {
        {
            let p1 = $p1;
            let p2 = $p2;
            crate::feq!(p1.x, p2.x) && crate::feq!(p1.y, p2.y)
        }
    }
}

#[macro_export]
macro_rules! poly {
    (
        [$x0:literal, $y0:literal],
        [$x1:literal, $y1:literal],
        $( [$xN:literal, $yN:literal] ),+
        $(,)? // Optional single trailing comma
    ) => {
        $crate::geo::shape::PathBuilder::new()
            .do_moveto(::nalgebra::point![$x0, $y0]).unwrap()
            .do_lineto(::nalgebra::point![$x1, $y1]).unwrap()
            $( .do_lineto(::nalgebra::point![$xN, $yN]).unwrap() )+
            .into_convex_polygon(1.0).unwrap()
    }
}

#[macro_export]
macro_rules! edge {
    (
        $x1:literal, $y1:literal, $x2:literal, $y2:literal $(,)?
    ) => {
        Edge::from((::nalgebra::point![$x1, $y1], ::nalgebra::point![$x2, $y2]))
    }
}
