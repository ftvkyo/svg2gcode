use geo::{BooleanOps, Intersects, MultiPolygon};

pub fn polygons_unite(mut polygons: MultiPolygon) -> MultiPolygon {
    let mut result = vec![];

    let find_next = |polygons: &MultiPolygon, current: &MultiPolygon| -> Option<usize> {
        polygons.iter()
            .enumerate()
            .find_map(|(i, p)| if p.intersects(current) { Some(i) } else { None })
    };

    while let Some(p_leader) = polygons.0.pop() {
        let mut p_leader = MultiPolygon::from(p_leader);

        while let Some(pi) = find_next(&polygons, &p_leader) {
            let p = polygons.0.remove(pi);
            p_leader = p_leader.union(&p);
        }

        result.extend(p_leader.into_iter());
    }

    MultiPolygon::from(result)
}
