use criterion::{Criterion, black_box, criterion_group, criterion_main};
use panopticon_optimizer::pareto::{Solution, compute_pareto_front};

fn bench_pareto_front(c: &mut Criterion) {
    let mut group = c.benchmark_group("pareto_front");

    for size in [10, 50, 100, 500] {
        group.bench_with_input(
            criterion::BenchmarkId::new("compute", size),
            &size,
            |b, &n| {
                let solutions: Vec<Solution> = (0..n)
                    .map(|i| {
                        let x = (i as f64) / (n as f64);
                        Solution::new(format!("s{i}"), vec![x, 1.0 - x, 0.5])
                    })
                    .collect();

                b.iter(|| {
                    let s = solutions.clone();
                    black_box(compute_pareto_front(s));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_pareto_front);
criterion_main!(benches);
