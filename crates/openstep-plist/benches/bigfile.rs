use criterion::{criterion_group, criterion_main, Criterion};
use openstep_plist::PlistParser;
use std::fs;

fn criterion_benchmark(c: &mut Criterion) {
    let s = fs::read_to_string("data/Truculenta.glyphs").unwrap();
    c.bench_function("Truculenta", |b| {
        b.iter(|| PlistParser::parse(s.clone(), false).expect("Whatever"))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
