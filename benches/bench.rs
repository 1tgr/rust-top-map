#![deny(warnings)]
#![deny(unused_extern_crates)]

extern crate criterion;
extern crate top_map;

use criterion::{criterion_group, criterion_main, Bencher, Criterion, Fun};
use std::collections::BTreeMap;
use top_map::TopMap;

fn bench(c: &mut Criterion) {
    fn insert_remove_empty_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = TopMap::new(100);
        m.reserve(1);

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
        let mut m = TopMap::new(100);
        m.extend((0..1000).map(|n| (n as isize, n)));

        b.iter(|| {
            m.insert(index, index);
            m.remove(index);
        });
    }

    fn insert_remove_existing_btree_map(b: &mut Bencher, &index: &isize) {
        let mut m = BTreeMap::new();
        m.extend((0..1000).map(|n| (n as isize, n)));

        b.iter(|| {
            m.insert(index, index);
            m.remove(&index);
        });
    }

    fn lookup_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = TopMap::new(100);
        m.extend((0..1000).map(|n| (n as isize, n)));

        b.iter(|| {
            assert_eq!(index, m[index]);
        });
    }

    fn lookup_btree_map(b: &mut Bencher, &index: &isize) {
        let mut m = BTreeMap::new();
        m.extend((0..1000).map(|n| (n as isize, n)));

        b.iter(|| {
            assert_eq!(index, m[&index]);
        });
    }

    fn increment_top_map(b: &mut Bencher, &index: &isize) {
        let mut m = TopMap::new(100);
        m.extend((0..1000).map(|n| (n as isize, n)));

        b.iter(|| {
            m[index] += 1;
        });
    }

    fn increment_btree_map(b: &mut Bencher, &index: &isize) {
        let mut m = BTreeMap::new();
        m.extend((0..1000).map(|n| (n as isize, n)));

        b.iter(|| {
            *m.get_mut(&index).unwrap() += 1;
        });
    }

    for &n in [0, 50, 99, 100, 500, 999].iter() {
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
}

criterion_group!(benches, bench);
criterion_main!(benches);
