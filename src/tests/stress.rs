use anyhow::Result;
use geo::Coord;
use svg::node::element;

use super::*;

#[test]
fn contour_joining() -> Result<()> {
    let dim = (160, 160);
    let mut g = element::Group::new();

    let w: f64 = 0.5;

    let n = 16;

    for i in 0..=n {
        let c1 = (i * 10) as f64;
        let c2 = (n * 10) as f64 - c1;

        g = g.add(make_line(vec![
            Coord { x: c1, y: 0.0 },
            Coord { x: 0.0, y: c2 },
        ], w));

        g = g.add(make_line(vec![
            Coord { x: dim.0 as f64, y: c1 },
            Coord { x: c2, y: dim.1 as f64 },
        ], w));
    }

    g = g.add(make_line(vec![
        Coord { x: 150.0, y: 10.0 },
        Coord { x: 10.0, y: 150.0 },
    ], w));

    g = g.add(make_circle(Coord { x: 40.0, y: 120.0 }, 10.0));
    g = g.add(make_circle(Coord { x: 120.0, y: 40.0 }, 10.0));

    g = g.add(make_polygon(vec![
        Coord { x: 60.0, y: 80.0 },
        Coord { x: 80.0, y: 60.0 },
        Coord { x: 100.0, y: 80.0 },
        Coord { x: 80.0, y: 100.0 },
    ]));

    let doc = make_test_svg(g, dim);
    run("stress-contour-joining", &doc, None)?;
    run("stress-contour-joining-offset", &doc, Some(1.0))?;

    Ok(())
}
