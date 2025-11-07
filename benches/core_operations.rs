use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use kombrucha::normalize_path;
use std::path::Path;

fn bench_normalize_path(c: &mut Criterion) {
    let test_paths = vec![
        Path::new("foo/bar/../baz"),
        Path::new("./foo/./bar"),
        Path::new("foo/../../bar"),
        Path::new("/usr/local/bin/../lib"),
        Path::new("a/b/c/../../d"),
    ];

    c.bench_function("normalize_path", |b| {
        b.iter(|| {
            for path in &test_paths {
                let _ = normalize_path(black_box(path));
            }
        })
    });
}

fn bench_normalize_path_single(c: &mut Criterion) {
    c.bench_function("normalize_path single", |b| {
        b.iter(|| {
            normalize_path(black_box(Path::new("foo/bar/../baz/./qux")))
        })
    });
}

fn bench_list_installed(c: &mut Criterion) {
    c.bench_function("list_installed", |b| {
        b.iter(|| {
            // This reads all receipts from /opt/homebrew/Cellar
            let _ = black_box(kombrucha::cellar::list_installed());
        })
    });
}

fn bench_normalize_path_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_path_complexity");

    let simple = Path::new("foo/bar/baz");
    let medium = Path::new("foo/bar/../baz/./qux");
    let complex = Path::new("a/b/c/../../d/e/f/../g/./h/../i");

    group.bench_with_input(BenchmarkId::new("simple", 0), &simple, |b, path| {
        b.iter(|| normalize_path(black_box(path)))
    });

    group.bench_with_input(BenchmarkId::new("medium", 1), &medium, |b, path| {
        b.iter(|| normalize_path(black_box(path)))
    });

    group.bench_with_input(BenchmarkId::new("complex", 2), &complex, |b, path| {
        b.iter(|| normalize_path(black_box(path)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_normalize_path,
    bench_normalize_path_single,
    bench_normalize_path_complexity,
    bench_list_installed
);
criterion_main!(benches);
