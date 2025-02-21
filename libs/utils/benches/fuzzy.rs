#![allow(unused_crate_dependencies)]
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use utils::fuzzy::Search;

fn bench_search(c: &mut Criterion) {
    let names = all_names();
    let search = create_search(&names);
    let mut index = 0usize;

    c.bench_function("search_exact", |b| {
        b.iter(|| {
            let name = names[index];
            index = index.wrapping_add(1) % names.len();

            search.search(black_box(name))
        })
    });

    c.bench_function("search_nonsense", |b| {
        b.iter(|| {
            let name = "dhbwuadsrasfdv";
            search.search(black_box(name))
        })
    });
}

fn all_names() -> Vec<&'static str> {
    include_str!("fuzzy_names.txt")
        .lines()
        .filter(|s| !s.is_empty())
        .collect()
}

fn create_search(names: &[&str]) -> Search<()> {
    let mut search = Search::<()>::new();

    for n in names {
        search.insert(n, ());
    }

    search.shrink_to_fit();
    search
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
