use super::edge::Edge;

pub fn fmt_edges(edges: impl Iterator<Item = Edge>) -> String {
    let mut s = String::new();
    for edge in edges {
        s.push_str(&format!("{edge}\n"));
    }
    if s.is_empty() {
        s.push('∅');
    }
    s
}

pub fn init_test_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp(None)
        .format_target(false)
        .is_test(true)
        .try_init();
}
