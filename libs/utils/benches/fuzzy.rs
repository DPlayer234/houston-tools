#![allow(unused_crate_dependencies)]
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use utils::fuzzy::Search;

fn bench_search(c: &mut Criterion) {
    let names = all_names();
    let search = create_search(&names);
    let nonsense_block = nonsense_block();

    let routine = |name| search.search(name);

    let mut index = 0usize;
    c.bench_function("search_exact", |b| {
        let setup = || {
            index = index.wrapping_add(1) % names.len();
            names[index]
        };

        b.iter_batched(setup, routine, BatchSize::SmallInput)
    });

    let mut index = 0usize;
    c.bench_function("search_nonsense", |b| {
        // gets a new "random" str out of the block of text
        let setup = || {
            loop {
                index = index.wrapping_add(1);

                let offset = index.wrapping_mul(11) % nonsense_block.len();
                let len = 8 + index.wrapping_mul(7) % 16;
                let range = offset..(offset + len);

                if let Some(name) = nonsense_block.get(range) {
                    return name;
                }
            }
        };

        b.iter_batched(setup, routine, BatchSize::SmallInput)
    });
}

fn names_file() -> &'static str {
    include_str!("fuzzy_names.txt")
}

fn all_names() -> Vec<&'static str> {
    names_file().lines().filter(|s| !s.is_empty()).collect()
}

fn nonsense_block() -> String {
    let mut iter = names_file()
        .lines()
        .filter(|s| !s.is_empty())
        .flat_map(|x| x.as_bytes().windows(4))
        .flat_map(str::from_utf8);

    // scramble the input chunks
    let mut chunks = Vec::new();
    chunks.extend(iter.next());
    for chunk in iter {
        let index = chunks.len().wrapping_add(7917).wrapping_mul(5087) % chunks.len();
        chunks.insert(index, chunk);
    }

    chunks.into_iter().collect()
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
