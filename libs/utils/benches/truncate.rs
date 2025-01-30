#![allow(unused_crate_dependencies)]
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Bencher, Criterion};

const TEXT: &str = "ヴァンプライ: Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum dolore eu feugiat nulla facilisis at vero eros et accumsan et iusto odio dignissim qui blandit praesent luptatum zzril delenit augue duis dolore te feugait nulla facilisi. Lorem ipsum dolor sit amet, consectetuer adipiscing elit, sed diam nonummy nibh euismod tincidunt ut laoreet dolore magna aliquam erat volutpat. Ut wisi enim ad minim veniam, quis nostrud exerci tation ullamcorper suscipit lobortis nisl ut aliquip ex ea commodo consequat. Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum dolore eu feugiat nulla facilisis at vero eros et accumsan et iusto odio dignissim qui blandit praesent luptatum zzril delenit augue duis dolore te feugait nulla facilisi. Nam liber tempor cum soluta nobis eleifend option congue nihil imperdiet doming id quod mazim placerat facer possim assum. Lorem ipsum dolor sit amet, consectetuer adipiscing elit, sed diam nonummy nibh euismod tincidunt ut laoreet dolore magna aliquam erat volutpat. Ut wisi enim ad minim veniam, quis nostrud exerci tation ullamcorper suscipit lobortis nisl ut aliquip ex ea commodo consequat. Duis autem vel eum iriure dolor in hendrerit in vulputate velit esse molestie consequat, vel illum dolore eu feugiat nulla facilisis. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur.";
const _: () = assert!(TEXT.len() == 2432, "expected this byte len");

fn bench_char_indices(c: &mut Criterion) {
    #[inline]
    fn find_truncate_at(s: &str, len: usize) -> Option<usize> {
        assert!(len >= 1, "cannot truncate to less than 1 character");

        if s.len() <= len {
            return None;
        }

        let mut indices = s.char_indices();
        let (end_at, _) = indices.nth(len - 1)?;
        indices.next()?;
        Some(end_at)
    }

    fn bench(len: usize) -> impl Fn(&mut Bencher<'_>) {
        move |b| b.iter(|| find_truncate_at(black_box(TEXT), black_box(len)))
    }

    c.bench_function("char_indices_trunc_short", bench(5));
    c.bench_function("char_indices_trunc_mid", bench(300));
    c.bench_function("char_indices_trunc_long", bench(2000));
    c.bench_function("char_indices_no_trunc", bench(2430));
}

fn bench_indices(c: &mut Criterion) {
    #[path = "../src/private/str.rs"]
    mod internals;

    #[inline]
    fn find_truncate_at(s: &str, len: usize) -> Option<usize> {
        assert!(len >= 1, "cannot truncate to less than 1 character");

        if s.len() <= len {
            return None;
        }

        let mut indices = internals::Indices::new(s);
        let end_at = indices.nth(len - 1)?;
        indices.next()?;
        Some(end_at)
    }

    fn bench(len: usize) -> impl Fn(&mut Bencher<'_>) {
        move |b| b.iter(|| find_truncate_at(black_box(TEXT), black_box(len)))
    }

    c.bench_function("indices_trunc_short", bench(5));
    c.bench_function("indices_trunc_mid", bench(300));
    c.bench_function("indices_trunc_long", bench(2000));
    c.bench_function("indices_no_trunc", bench(2430));
}

criterion_group!(benches, bench_char_indices, bench_indices);
criterion_main!(benches);
