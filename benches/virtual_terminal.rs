use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use dre::bench::{self, VirtualTerminal};

const PLACEMENTS: [usize; 4] = [10, 100, 1000, 4000];
const LARGEST_SAMPLE_SIZE: usize = 10;

fn warmed(frame: &[bench::Desired]) -> VirtualTerminal {
    let mut terminal = VirtualTerminal::default();
    bench::commit(&mut terminal, frame, &mut || bench::sprite_content());
    terminal
}

fn cold(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold");
    for placements in PLACEMENTS {
        let frame = bench::frame(placements);
        group.throughput(Throughput::Elements(placements as u64));
        if placements == *PLACEMENTS.last().unwrap() {
            group.sample_size(LARGEST_SAMPLE_SIZE);
        }
        group.bench_with_input(
            BenchmarkId::from_parameter(placements),
            &frame,
            |b, frame| {
                b.iter_batched(
                    VirtualTerminal::default,
                    |mut terminal| {
                        black_box(bench::commit(&mut terminal, frame, &mut || {
                            bench::sprite_content()
                        }))
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn identical(c: &mut Criterion) {
    let mut group = c.benchmark_group("identical");
    for placements in PLACEMENTS {
        let frame = bench::frame(placements);
        let mut terminal = warmed(&frame);
        group.throughput(Throughput::Elements(placements as u64));
        if placements == *PLACEMENTS.last().unwrap() {
            group.sample_size(LARGEST_SAMPLE_SIZE);
        }
        group.bench_function(BenchmarkId::from_parameter(placements), |b| {
            b.iter(|| {
                black_box(bench::commit(&mut terminal, &frame, &mut || {
                    bench::sprite_content()
                }))
            });
        });
    }
    group.finish();
}

fn one_moved(c: &mut Criterion) {
    let mut group = c.benchmark_group("one moved");
    for placements in PLACEMENTS {
        let a = bench::frame(placements);
        let b_frame = bench::frame_with_one_moved(placements);
        let mut terminal = warmed(&a);
        group.throughput(Throughput::Elements(placements as u64));
        if placements == *PLACEMENTS.last().unwrap() {
            group.sample_size(LARGEST_SAMPLE_SIZE);
        }
        group.bench_function(BenchmarkId::from_parameter(placements), |b| {
            // Alternating A and B keeps every commit a genuine one-placement
            // delta against the frame the terminal currently holds, with no
            // state restore inside the timed region.
            let mut next_is_b = true;
            b.iter(|| {
                let frame = if next_is_b { &b_frame } else { &a };
                next_is_b = !next_is_b;
                black_box(bench::commit(&mut terminal, frame, &mut || {
                    bench::sprite_content()
                }))
            });
        });
    }
    group.finish();
}

criterion_group!(benches, cold, identical, one_moved);
criterion_main!(benches);
