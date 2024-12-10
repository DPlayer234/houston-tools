#![allow(unused_crate_dependencies)]
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use smallvec::SmallVec;
use utils::str_as_data::{decode_b65536, to_b65536};

fn bench_to_b65536(c: &mut Criterion) {
    fn bench(c: &mut Criterion, name: &str, data: &[u8]) {
        c.bench_function(name, |b| b.iter(|| to_b65536(black_box(data))));
    }

    bench(c, "to_b65536_small", &create_data::<16>());
    bench(c, "to_b65536_large", &create_data::<12000>());
}

fn bench_from_b65536(c: &mut Criterion) {
    fn bench(c: &mut Criterion, name: &str, data: &[u8]) {
        let data = to_b65536(data);

        c.bench_function(name, |b| {
            b.iter(|| {
                let mut vec = <SmallVec<[u8; 16]>>::new();
                black_box(decode_b65536(&mut vec, &data)).expect("data is valid");
                vec
            })
        });
    }

    bench(c, "from_b65536_small", &create_data::<16>());
    bench(c, "from_b65536_large", &create_data::<12000>());
}

fn create_data<const LEN: usize>() -> [u8; LEN] {
    let mut buf = [0u8; LEN];
    let (_, main, _) = unsafe { buf.align_to_mut::<u16>() };

    #[allow(clippy::cast_possible_truncation)]
    for (index, b) in main.iter_mut().enumerate() {
        *b = u16::MAX - index as u16;
    }

    buf
}

criterion_group!(b65536, bench_to_b65536, bench_from_b65536);
criterion_main!(b65536);
