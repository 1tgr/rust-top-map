#![deny(warnings)]
#![deny(unused_extern_crates)]

extern crate criterion;
extern crate top_map;

use criterion::{criterion_group, criterion_main, Bencher, Criterion, Fun};
use std::collections::BTreeMap;
use top_map::TopMap;

fn bench(c: &mut Criterion) {
    fn insert_remove_empty_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = TopMap::<[Option<(isize, isize)>; 128]>::new();

        b.iter(|| {
            m.insert(index, index);
            m.remove(index);
        });
    }

    fn insert_remove_empty_btree_map(b: &mut Bencher, &index: &isize) {
        let mut m = BTreeMap::new();

        b.iter(|| {
            m.insert(index, index);
            m.remove(&index);
        });
    }

    fn insert_remove_existing_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = (0..1000)
            .map(|n| (n as isize, n))
            .collect::<TopMap<[Option<(isize, isize)>; 128]>>();

        m.shrink_to_fit();

        b.iter(|| {
            m.insert(index, index);
            m.remove(index);
        });
    }

    fn insert_remove_existing_btree_map(b: &mut Bencher, &index: &isize) {
        let mut m = (0..1000).map(|n| (n as isize, n)).collect::<BTreeMap<isize, isize>>();

        b.iter(|| {
            m.insert(index, index);
            m.remove(&index);
        });
    }

    fn lookup_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = (0..1000)
            .map(|n| (n as isize, n))
            .collect::<TopMap<[Option<(isize, isize)>; 128]>>();

        m.shrink_to_fit();

        b.iter(|| {
            assert_eq!(index, m[index]);
        });
    }

    fn lookup_btree_map(b: &mut Bencher, &index: &isize) {
        let m = (0..1000).map(|n| (n as isize, n)).collect::<BTreeMap<isize, isize>>();

        b.iter(|| {
            assert_eq!(index, m[&index]);
        });
    }

    fn increment_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = (0..1000)
            .map(|n| (n as isize, n))
            .collect::<TopMap<[Option<(isize, isize)>; 128]>>();

        m.shrink_to_fit();

        b.iter(|| {
            m[index] += 1;
        });
    }

    fn increment_btree_map(b: &mut Bencher, &index: &isize) {
        let mut m = (0..1000).map(|n| (n as isize, n)).collect::<BTreeMap<isize, isize>>();

        b.iter(|| {
            *m.get_mut(&index).unwrap() += 1;
        });
    }

    fn extend_in_direction(b: &mut Bencher, &direction: &i8) {
        b.iter(|| {
            let _: TopMap<[Option<(isize, isize)>; 128]> = if direction > 0 {
                (0..1000).map(|n| (n as isize, n)).collect()
            } else {
                (0..1000).rev().map(|n| (n as isize, n)).collect()
            };
        });
    }

    let indices = vec![0, 50, 63, 64, 65, 127, 128, 129, 999];

    for &n in indices.iter() {
        c.bench_functions(
            &n.to_string(),
            vec![
                Fun::new("insert_remove_empty_top_map", insert_remove_empty_top_map),
                Fun::new("insert_remove_existing_top_map", insert_remove_existing_top_map),
                Fun::new("lookup_top_map", lookup_top_map),
                Fun::new("increment_top_map", increment_top_map),
                Fun::new("insert_remove_empty_btree_map", insert_remove_empty_btree_map),
                Fun::new("insert_remove_existing_btree_map", insert_remove_existing_btree_map),
                Fun::new("lookup_btree_map", lookup_btree_map),
                Fun::new("increment_btree_map", increment_btree_map),
            ],
            n,
        );
    }

    c.bench_function_over_inputs(
        "insert_remove_empty_top_map",
        insert_remove_empty_top_map,
        indices.clone(),
    );
    c.bench_function_over_inputs(
        "insert_remove_existing_top_map",
        insert_remove_existing_top_map,
        indices.clone(),
    );
    c.bench_function_over_inputs("lookup_top_map", lookup_top_map, indices.clone());
    c.bench_function_over_inputs("increment_top_map", increment_top_map, indices.clone());
    c.bench_function_over_inputs(
        "insert_remove_empty_btree_map",
        insert_remove_empty_btree_map,
        indices.clone(),
    );
    c.bench_function_over_inputs(
        "insert_remove_existing_btree_map",
        insert_remove_existing_btree_map,
        indices.clone(),
    );
    c.bench_function_over_inputs("lookup_btree_map", lookup_btree_map, indices.clone());
    c.bench_function_over_inputs("increment_btree_map", increment_btree_map, indices.clone());
    c.bench_function_over_inputs("extend_in_direction", extend_in_direction, vec![-1, 1]);
}

criterion_group!(benches, bench);
criterion_main!(benches);
