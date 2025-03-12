use super::*;

#[test]
fn line_joining() -> Result<()> {
    let dim = (100, 100);
    let g = element::Group::new()
        .add(make_line(vec![
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 30.0, y: 50.0 },
        ], 5.0))
        .add(make_line(vec![
            Coord { x: 30.0, y: 50.0 },
            Coord { x: 70.0, y: 50.0 },
        ], 5.0))
        .add(make_line(vec![
            Coord { x: 70.0, y: 50.0 },
            Coord { x: 90.0, y: 90.0 },
        ], 5.0));

    let doc = make_test_svg(g, dim);
    run("concept-line-joining", &doc, None)?;
    run("concept-line-joining-offset", &doc, Some(5.0))?;

    Ok(())
}

#[test]
fn line_contour_merging() -> Result<()> {
    let dim = (100, 100);
    let g = element::Group::new()
        .add(make_line(vec![
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 30.0, y: 50.0 },
            Coord { x: 70.0, y: 50.0 },
            Coord { x: 90.0, y: 90.0 },
        ], 5.0))
        .add(make_line(vec![
            Coord { x: 90.0, y: 10.0 },
            Coord { x: 10.0, y: 90.0 },
        ], 5.0));

    let doc = make_test_svg(g, dim);
    run("concept-line-contour-merging", &doc, None)?;
    run("concept-line-contour-merging-offset", &doc, Some(5.0))?;

    Ok(())
}

#[test]
fn line_contour_merging_with_interior() -> Result<()> {
    let dim = (100, 100);
    let g = element::Group::new()
        .add(make_line(vec![
            Coord { x: 20.0, y: 20.0 },
            Coord { x: 20.0, y: 80.0 },
            Coord { x: 80.0, y: 80.0 },
            Coord { x: 80.0, y: 20.0 },
        ], 5.0))
        .add(make_line(vec![
            Coord { x: 10.0, y: 50.0 },
            Coord { x: 90.0, y: 50.0 },
        ], 5.0));

    let doc = make_test_svg(g, dim);
    run("concept-line-contour-merging-with-interior", &doc, None)?;
    run("concept-line-contour-merging-with-interior-offset", &doc, Some(5.0))?;

    Ok(())
}
