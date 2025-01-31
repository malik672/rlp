use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

#[inline(always)]
fn optimized_copy_from_slice<T>(dst: &mut [T], src: &[T]) 
where
    T: Copy,
{
    assert_eq!(dst.len(), src.len(), "source and destination slices must have equal lengths");
    unsafe {
        let mut src_ptr = src.as_ptr();
        let mut dst_ptr = dst.as_mut_ptr();
        let end_ptr = src_ptr.add(src.len());
        
        while src_ptr < end_ptr {
            *dst_ptr = *src_ptr;
            src_ptr = src_ptr.add(1);
            dst_ptr = dst_ptr.add(1);
        }
    }
}

fn bench_copies(c: &mut Criterion) {
    let sizes = [4, 8, 16, 32, 64, 128, 256];
    let mut group = c.benchmark_group("slice_copy");

    for size in sizes.iter() {
        let src: Vec<u8> = (0..*size).map(|i| i as u8).collect();
        
        group.bench_with_input(BenchmarkId::new("copy_from_slice", size), size, |b, &size| {
            let mut dst = vec![0u8; size];
            b.iter(|| {
                dst.copy_from_slice(black_box(&src));
                black_box(&dst);
            });
        });
        group.bench_with_input(BenchmarkId::new("optimized_copy", size), size, |b, &size| {
            let mut dst = vec![0u8; size];
            b.iter(|| {
                optimized_copy_from_slice(black_box(&mut dst), black_box(&src));
                black_box(&dst);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_copies);
criterion_main!(benches);