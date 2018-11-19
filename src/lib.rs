#![deny(warnings)]
#![deny(unused_extern_crates)]

extern crate fixed_vec_deque;

use std::collections::BTreeMap;
use std::collections::btree_map;
use std::fmt;
use std::mem;
use std::ops;

use fixed_vec_deque::{Array as FvdArray, FixedVecDeque};

pub trait Array {
    type Key;
    type Value;
    type Array: FvdArray<Item = Option<(Self::Key, Self::Value)>>;
}

impl<Key, Value, A> Array for A
where
    A: FvdArray<Item = Option<(Key, Value)>>,
{
    type Key = Key;
    type Value = Value;
    type Array = Self;
}

pub struct TopMap<A>
where
    A: Array,
{
    top: FixedVecDeque<A::Array>,
    rest: BTreeMap<A::Key, A::Value>,
}

impl<A> TopMap<A>
where
    A: Array,
    A::Key: Ord,
{
    pub fn new() -> Self {
        Self {
            top: FixedVecDeque::new(),
            rest: BTreeMap::new(),
        }
    }
}

fn positive(i: isize) -> Option<usize> {
    if i >= 0 {
        Some(i as usize)
    } else {
        None
    }
}

enum Index {
    AboveTop { distance: usize },
    InsideTop { index: usize },
    Rest,
}

pub enum Entry<'a, A: 'a>
where
    A: Array,
{
    AboveTop {
        key: A::Key,
        map: &'a mut TopMap<A>,
        distance: usize,
    },

    Vec(A::Key, &'a mut Option<(A::Key, A::Value)>),
    BTreeMap(btree_map::Entry<'a, A::Key, A::Value>),
}

impl<'a, A> Entry<'a, A>
where
    A: Array,
    A::Key: Ord,
{
    fn insert(self, value: A::Value) -> Option<A::Value> {
        match self {
            Entry::AboveTop { key, map, distance } => {
                *map.insert_above_top(distance) = Some((key, value));
                None
            }

            Entry::Vec(key, entry) => Some(mem::replace(entry, Some((key, value)))?.1),
            Entry::BTreeMap(btree_map::Entry::Occupied(mut entry)) => Some(entry.insert(value)),

            Entry::BTreeMap(btree_map::Entry::Vacant(entry)) => {
                entry.insert(value);
                None
            }
        }
    }

    pub fn or_insert(self, default: A::Value) -> &'a mut A::Value {
        match self {
            Entry::AboveTop { key, map, distance } => {
                &mut map.insert_above_top(distance).get_or_insert((key, default)).1
            }

            Entry::Vec(key, entry) => &mut entry.get_or_insert((key, default)).1,
            Entry::BTreeMap(entry) => entry.or_insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> A::Value>(self, default: F) -> &'a mut A::Value {
        match self {
            Entry::AboveTop { key, map, distance } => {
                &mut map.insert_above_top(distance).get_or_insert_with(|| (key, default())).1
            }

            Entry::Vec(key, entry) => &mut entry.get_or_insert_with(|| (key, default())).1,
            Entry::BTreeMap(entry) => entry.or_insert_with(default),
        }
    }
}

impl<A> TopMap<A>
where
    A: Array,
{
    pub fn len(&self) -> usize {
        self.top.iter().filter(|&entry| entry.is_some()).count() + self.rest.len()
    }
}

fn ensure_index<T, A>(v: &mut FixedVecDeque<A>, index: usize) -> &mut Option<T>
where
    A: FvdArray<Item = Option<T>>,
{
    if let Some(count) = (index + 1).checked_sub(v.len()) {
        for _ in 0..count {
            *v.push_back() = None;
        }
    }

    &mut v[index]
}

impl<A> TopMap<A>
where
    A: Array,
    A::Key: Ord,
{
    fn insert_above_top(&mut self, distance: usize) -> &mut Option<(A::Key, A::Value)> {
        if let Some(new_count) = A::Array::size().checked_sub(distance) {
            assert!(new_count <= self.top.len());

            let drain_count = self.top.len() - new_count;
            for _ in 0..drain_count {
                if let Some((key, value)) = mem::replace(self.top.pop_back().unwrap(), None) {
                    self.rest.insert(key, value);
                }
            }

            for _ in 0..distance - 1 {
                *self.top.push_front() = None;
            }
        } else {
            while let Some(entry) = self.top.pop_back() {
                if let Some((key, value)) = mem::replace(entry, None) {
                    self.rest.insert(key, value);
                }
            }
        }

        let front = self.top.push_front();
        *front = None;
        front
    }
}

impl<A> TopMap<A>
where
    A: Array,
    A::Key: Copy + Ord,
    isize: From<A::Key>,
{
    pub fn iter(&self) -> impl Iterator<Item = (A::Key, &A::Value)> {
        self.top
            .iter()
            .filter_map(|entry| entry.as_ref().map(|(key, value)| (*key, value)))
            .chain(self.rest.iter().map(|(key, value)| (*key, value)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (A::Key, &mut A::Value)> {
        self.top
            .iter_mut()
            .filter_map(|entry| entry.as_mut().map(|(key, value)| (*key, value)))
            .chain(self.rest.iter_mut().map(|(key, value)| (*key, value)))
    }

    pub fn clear(&mut self) {
        self.top.clear();
        self.rest.clear();
    }

    fn index(&self, key: A::Key) -> Index {
        let index = if let Some(ref min_entry) = self.top.front() {
            let &(min_key, _) = min_entry.as_ref().expect("top entry should be filled");
            isize::from(key) - isize::from(min_key)
        } else {
            0
        };

        if let Some(index) = positive(index) {
            if index >= A::Array::size() {
                Index::Rest
            } else {
                Index::InsideTop { index }
            }
        } else {
            Index::AboveTop {
                distance: -index as usize,
            }
        }
    }

    pub fn entry(&mut self, key: A::Key) -> Entry<A> {
        match self.index(key) {
            Index::AboveTop { distance } => Entry::AboveTop {
                key,
                map: self,
                distance,
            },

            Index::InsideTop { index } => Entry::Vec(key, ensure_index(&mut self.top, index)),
            Index::Rest => Entry::BTreeMap(self.rest.entry(key)),
        }
    }

    pub fn get(&self, key: A::Key) -> Option<&A::Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,
            Index::InsideTop { index } => Some(&self.top.get(index)?.as_ref()?.1),
            Index::Rest => self.rest.get(&key),
        }
    }

    pub fn get_mut(&mut self, key: A::Key) -> Option<&mut A::Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,
            Index::InsideTop { index } => Some(&mut self.top.get_mut(index)?.as_mut()?.1),
            Index::Rest => self.rest.get_mut(&key),
        }
    }

    pub fn insert(&mut self, key: A::Key, value: A::Value) -> Option<A::Value> {
        self.entry(key).insert(value)
    }

    pub fn remove(&mut self, key: A::Key) -> Option<A::Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,

            Index::InsideTop { index: 0 } => {
                let (_, value) = mem::replace(self.top.pop_front()?, None)?;

                while let Some(None) = self.top.front() {
                    self.top.pop_front();
                }

                let min_top_key = if let Some(&Some((min_top_key, _))) = self.top.front() {
                    Some(min_top_key)
                } else if let Some((&rest_key, _)) = self.rest.iter().next() {
                    let rest_value = self.rest.remove(&rest_key).unwrap();
                    *self.top.push_back() = Some((rest_key, rest_value));
                    Some(rest_key)
                } else {
                    None
                };

                if let Some(min_top_key) = min_top_key {
                    while let Some((&key, _)) = self.rest.iter().next() {
                        let index = positive(isize::from(key) - isize::from(min_top_key)).expect(
                            "everything in the rest map should have an index higher than everything in the top vec",
                        );

                        if index >= A::Array::size() {
                            break;
                        }

                        let value = self.rest.remove(&key).unwrap();
                        *ensure_index(&mut self.top, index) = Some((key, value));
                    }
                }

                Some(value)
            }

            Index::InsideTop { index } => {
                let (_, value) = mem::replace(self.top.get_mut(index)?, None)?;
                Some(value)
            }

            Index::Rest => self.rest.remove(&key),
        }
    }
}

impl<A> ops::Index<A::Key> for TopMap<A>
where
    A: Array,
    A::Key: Copy + Ord + fmt::Debug,
    isize: From<A::Key>,
{
    type Output = A::Value;

    fn index(&self, index: A::Key) -> &A::Value {
        self.get(index)
            .unwrap_or_else(|| panic!("no item with key {:?}", index))
    }
}

impl<A> ops::IndexMut<A::Key> for TopMap<A>
where
    A: Array,
    A::Key: Copy + Ord + fmt::Debug,
    isize: From<A::Key>,
{
    fn index_mut(&mut self, index: A::Key) -> &mut A::Value {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("no item with key {:?}", index))
    }
}

impl<A> Extend<(A::Key, A::Value)> for TopMap<A>
where
    A: Array,
    A::Key: Copy + Ord,
    isize: From<A::Key>,
{
    fn extend<T: IntoIterator<Item = (A::Key, A::Value)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Array, TopMap};

    static ITEMS: &[(isize, &'static str)] = &[
        (100, "a1"),
        (101, "a2"),
        (200, "b1"),
        (201, "b2"),
        (300, "c1"),
        (301, "c2"),
    ];

    fn lens<A>(m: &TopMap<A>) -> [usize; 3]
    where
        A: Array,
    {
        [
            m.len(),
            m.top.iter().filter(|&entry| entry.is_some()).count(),
            m.rest.len(),
        ]
    }

    #[test]
    fn extend() {
        let mut m = TopMap::<[Option<(isize, &str)>; 10]>::new();
        m.extend(ITEMS.iter().cloned());
        assert_eq!([6, 2, 4], lens(&m));

        let items = m.iter()
            .map(|(key, &value)| (key, value))
            .collect::<Vec<(isize, &'static str)>>();

        assert_eq!(ITEMS, &items[..]);
    }

    #[test]
    fn insert() {
        let mut m = TopMap::<[Option<(isize, &str)>; 10]>::new();
        assert_eq!(None, m.get(100isize));
        assert_eq!([0, 0, 0], lens(&m));

        assert_eq!(None, m.insert(200, "b1"));
        assert_eq!([1, 1, 0], lens(&m));

        assert_eq!(None, m.insert(201, "b2"));
        assert_eq!([2, 2, 0], lens(&m));

        assert_eq!(None, m.insert(300, "c1"));
        assert_eq!([3, 2, 1], lens(&m));

        assert_eq!(None, m.insert(301, "c2"));
        assert_eq!([4, 2, 2], lens(&m));

        assert_eq!(None, m.insert(100, "a1"));
        assert_eq!([5, 1, 4], lens(&m));

        assert_eq!(None, m.insert(101, "a2"));
        assert_eq!([6, 2, 4], lens(&m));

        let items = m.iter()
            .map(|(key, &value)| (key, value))
            .collect::<Vec<(isize, &'static str)>>();

        assert_eq!(ITEMS, &items[..]);
    }

    #[test]
    fn remove() {
        let mut m = TopMap::<[Option<(isize, &str)>; 10]>::new();
        m.extend(ITEMS.iter().cloned());
        assert_eq!(None, m.remove(-1));
        assert_eq!([6, 2, 4], lens(&m));

        assert_eq!(Some("a1"), m.remove(100));
        assert_eq!([5, 1, 4], lens(&m));

        assert_eq!(Some("a2"), m.remove(101));
        assert_eq!([4, 2, 2], lens(&m));

        assert_eq!(Some("c1"), m.remove(300));
        assert_eq!([3, 2, 1], lens(&m));

        assert_eq!(Some("c2"), m.remove(301));
        assert_eq!([2, 2, 0], lens(&m));

        assert_eq!(Some("b1"), m.remove(200));
        assert_eq!([1, 1, 0], lens(&m));

        assert_eq!(Some("b2"), m.remove(201));
        assert_eq!([0, 0, 0], lens(&m));

        let items = m.iter()
            .map(|(key, &value)| (key, value))
            .collect::<Vec<(isize, &'static str)>>();

        assert!(items.is_empty());
    }

    #[test]
    fn insert_remove_existing_m1() {
        let mut m = TopMap::<[Option<(isize, isize)>; 128]>::new();
        m.extend((0..1000).map(|n| (n as isize, n)));
        assert_eq!([1000, 128, 872], lens(&m));

        let index = -1;
        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 128, 873], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 128, 872], lens(&m));

        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 128, 873], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 128, 872], lens(&m));
    }

    #[test]
    fn insert_remove_existing_m3() {
        let mut m = TopMap::<[Option<(isize, isize)>; 128]>::new();
        m.extend((0..1000).map(|n| (n as isize, n)));
        assert_eq!([1000, 128, 872], lens(&m));

        let index = -3;
        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 126, 875], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 128, 872], lens(&m));

        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 126, 875], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 128, 872], lens(&m));
    }

    #[test]
    fn insert_remove_existing_m999() {
        let mut m = TopMap::<[Option<(isize, isize)>; 128]>::new();
        m.extend((0..1000).map(|n| (n as isize, n)));
        assert_eq!([1000, 128, 872], lens(&m));

        let index = -999;
        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 1, 1000], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 128, 872], lens(&m));

        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 1, 1000], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 128, 872], lens(&m));
    }
}
