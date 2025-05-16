#![allow(unused_crate_dependencies)]
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use smallvec::SmallVec;
use utils::str_as_data::*;

fn bench_to_b65536(c: &mut Criterion) {
    fn bench(c: &mut Criterion, name: &str, data: &[u8]) {
        c.bench_function(name, |b| b.iter(|| b65536::to_string(black_box(data))));
    }

    bench(c, "to_b65536_small", &create_data::<16>());
    bench(c, "to_b65536_large", &create_data::<12000>());
}

fn bench_from_b65536(c: &mut Criterion) {
    fn bench(c: &mut Criterion, name: &str, data: &[u8]) {
        let data = b65536::to_string(data);

        c.bench_function(name, |b| {
            b.iter(|| {
                let mut vec = <SmallVec<[u8; 16]>>::new();
                black_box(b65536::decode(&mut vec, &data)).expect("data is valid");
                vec
            })
        });
    }

    bench(c, "from_b65536_small", &create_data::<16>());
    bench(c, "from_b65536_large", &create_data::<12000>());
}

fn bench_to_b20bit(c: &mut Criterion) {
    fn bench(c: &mut Criterion, name: &str, data: &[u8]) {
        c.bench_function(name, |b| b.iter(|| b20bit::to_string(black_box(data))));
    }

    bench(c, "to_b20bit_small", &create_data::<16>());
    bench(c, "to_b20bit_large", &create_data::<12000>());
}

fn bench_from_b20bit(c: &mut Criterion) {
    fn bench(c: &mut Criterion, name: &str, data: &[u8]) {
        let data = b20bit::to_string(data);

        c.bench_function(name, |b| {
            b.iter(|| {
                let mut vec = <SmallVec<[u8; 16]>>::new();
                black_box(b20bit::decode(&mut vec, &data)).expect("data is valid");
                vec
            })
        });
    }

    bench(c, "from_b20bit_small", &create_data::<16>());
    bench(c, "from_b20bit_large", &create_data::<12000>());
}

fn create_data<const LEN: usize>() -> [u8; LEN] {
    let mut buf = [0u8; LEN];

    // SAFETY: slice transmute between primitives
    let (_, main, _) = unsafe { buf.align_to_mut::<u16>() };

    #[expect(clippy::cast_possible_truncation)]
    for (index, b) in main.iter_mut().enumerate() {
        *b = u16::MAX - index as u16;
    }

    buf
}

criterion_group!(
    str_as_data,
    bench_to_b65536,
    bench_from_b65536,
    bench_to_b20bit,
    bench_from_b20bit
);
criterion_main!(str_as_data);
