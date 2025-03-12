use super::*;

#[test]
fn line() -> Result<()> {
    let dim = (100, 100);
    let g = element::Group::new()
        .add(make_line(vec![
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 90.0, y: 90.0 },
        ], 5.0));

    let doc = make_test_svg(g, dim);
    run("generated-line", &doc, None)?;

    Ok(())
}

#[test]
fn line_grid() -> Result<()> {
    let dim = (100, 100);
    let grid = (4, 4);

    let mut g = element::Group::new();

    for gx in 0..grid.0 {
        for gy in 0..grid.1 {
            let x1 = (gx * dim.0 / grid.0) as f64;
            let y1 = (gy * dim.1 / grid.1) as f64;
            let x2 = x1 + (dim.0 / grid.0) as f64;
            let y2 = y1 + (dim.1 / grid.1) as f64;

            let cell = gy * grid.0 + gx;

            g = g.add(make_line(vec![
                Coord { x: x1 + 5.0, y: y1 + 5.0 },
                Coord { x: x2 - 5.0, y: y2 - 5.0 },
            ], (cell + 1) as f64 / 4.0 ));
        }
    }

    let doc = make_test_svg(g, dim);
    run("generated-line-grid", &doc, None)?;
    run("generated-line-grid-offset", &doc, Some(5.0))?;

    Ok(())
}

#[test]
fn line_string_grid() -> Result<()> {
    let dim = (100, 100);
    let grid = (4, 4);

    let mut g = element::Group::new();

    for gx in 0..grid.0 {
        for gy in 0..grid.1 {
            let x1 = (gx * dim.0 / grid.0) as f64;
            let y1 = (gy * dim.1 / grid.1) as f64;
            let x2 = x1 + (dim.0 / grid.0) as f64;
            let y2 = y1 + (dim.1 / grid.1) as f64;

            let cell = gy * grid.0 + gx;

            g = g.add(make_line(vec![
                Coord { x: x1 + 5.0, y: y1 + 5.0 },
                Coord { x: x1 + 10.0, y: y1 + 15.0},
                Coord { x: x2 - 10.0, y: y2 - 15.0},
                Coord { x: x2 - 5.0, y: y2 - 5.0 },
            ], (cell + 1) as f64 / 4.0 ));
        }
    }

    let doc = make_test_svg(g, dim);
    run("generated-line-string-grid", &doc, None)?;
    run("generated-line-string-grid-offset", &doc, Some(2.0))?;

    Ok(())
}

#[test]
fn circle_grid() -> Result<()> {
    let dim = (100, 100);
    let grid = (4, 4);

    let mut g = element::Group::new();

    for gx in 0..grid.0 {
        for gy in 0..grid.1 {
            let x1 = (gx * dim.0 / grid.0) as f64;
            let y1 = (gy * dim.1 / grid.1) as f64;
            let x2 = x1 + (dim.0 / grid.0) as f64;
            let y2 = y1 + (dim.1 / grid.1) as f64;

            let cell = gy * grid.0 + gx;

            g = g.add(make_circle(Coord { x: (x1 + x2) / 2.0, y: (y1 + y2) / 2.0 }, (cell + 1) as f64 / 2.0 ));
        }
    }

    let doc = make_test_svg(g, dim);
    run("generated-circle-grid", &doc, None)?;
    run("generated-circle-grid-offset", &doc, Some(5.0))?;

    Ok(())
}

#[test]
fn polygon_grid() -> Result<()> {
    let dim = (100, 100);
    let grid = (4, 4);

    let mut g = element::Group::new();

    for gx in 0..grid.0 {
        for gy in 0..grid.1 {
            let x1 = (gx * dim.0 / grid.0) as f64 + 10.0;
            let y1 = (gy * dim.1 / grid.1) as f64 + 10.0;
            let x2 = ((gx + 1) * dim.0 / grid.0) as f64 - 10.0;
            let y2 = ((gy + 1) * dim.1 / grid.1) as f64 - 10.0;

            let cell = gy * grid.0 + gx;
            let d = cell as f64 / 8.0;

            g = g.add(make_polygon(vec![
                Coord { x: x1 - d, y: y1 - d },
                Coord { x: x1 - d, y: y2 + d },
                Coord { x: x2 + d, y: y2 + d },
                Coord { x: x2 + d, y: y1 - d },
            ]));
        }
    }

    let doc = make_test_svg(g, dim);
    run("generated-polygon-grid", &doc, None)?;
    run("generated-polygon-grid-offset", &doc, Some(5.0))?;

    Ok(())
}
