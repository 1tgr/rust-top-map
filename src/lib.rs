#![deny(warnings)]
#![deny(unused_extern_crates)]

use std::collections::{BTreeMap, VecDeque};
use std::collections::btree_map;
use std::fmt;
use std::mem;
use std::ops;

pub struct TopMap<Key, Value> {
    top_count: usize,
    first: Option<(Key, Value)>,
    top: VecDeque<Option<(Key, Value)>>,
    rest: BTreeMap<Key, Value>,
}

impl<Key, Value> TopMap<Key, Value>
where
    Key: Ord,
{
    pub fn new(top_count: usize) -> Self {
        Self {
            top_count,
            first: None,
            top: VecDeque::new(),
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
    First,
    InsideTop { index: usize },
    Rest,
}

pub enum Entry<'a, Key: 'a, Value: 'a> {
    AboveTop {
        key: Key,
        map: &'a mut TopMap<Key, Value>,
        distance: usize,
    },

    Vec {
        key: Key,
        entry: &'a mut Option<(Key, Value)>,
    },

    BTreeMap(btree_map::Entry<'a, Key, Value>),
}

impl<'a, Key, Value> Entry<'a, Key, Value>
where
    Key: Copy + Ord,
{
    fn insert(self, value: Value) -> Option<Value> {
        match self {
            Entry::AboveTop { key, map, distance } => {
                *map.insert_above_top(distance) = Some((key, value));
                None
            }

            Entry::Vec { key, entry } => Some(mem::replace(entry, Some((key, value)))?.1),
            Entry::BTreeMap(btree_map::Entry::Occupied(mut entry)) => Some(entry.insert(value)),

            Entry::BTreeMap(btree_map::Entry::Vacant(entry)) => {
                entry.insert(value);
                None
            }
        }
    }

    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::AboveTop { key, map, distance } => {
                &mut map.insert_above_top(distance).get_or_insert((key, default)).1
            }

            Entry::Vec { key, entry } => &mut entry.get_or_insert((key, default)).1,
            Entry::BTreeMap(entry) => entry.or_insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> Value>(self, default: F) -> &'a mut Value {
        match self {
            Entry::AboveTop { key, map, distance } => {
                &mut map.insert_above_top(distance).get_or_insert_with(|| (key, default())).1
            }

            Entry::Vec { key, entry } => &mut entry.get_or_insert_with(|| (key, default())).1,
            Entry::BTreeMap(entry) => entry.or_insert_with(default),
        }
    }
}

impl<Key, Value> TopMap<Key, Value> {
    pub fn len(&self) -> usize {
        (if self.first.is_some() { 1 } else { 0 }) + self.top.iter().filter(|&entry| entry.is_some()).count()
            + self.rest.len()
    }
}

fn ensure_index<T>(v: &mut VecDeque<Option<T>>, index: usize) -> &mut Option<T> {
    if let Some(count) = (index + 1).checked_sub(v.len()) {
        v.reserve(count);
        for _ in 0..count {
            v.push_back(None);
        }
    }

    &mut v[index]
}

impl<Key, Value> TopMap<Key, Value>
where
    Key: Ord,
{
    fn insert_above_top(&mut self, distance: usize) -> &mut Option<(Key, Value)> {
        if let Some(new_count) = self.top_count.checked_sub(distance) {
            self.rest.extend(self.top.drain(new_count..).filter_map(|entry| entry));

            for _ in 0..distance {
                self.top.push_front(None);
            }
        } else {
            self.rest.extend(self.top.drain(..).filter_map(|entry| entry));
        }

        self.first = None;
        &mut self.first
    }
}

impl<Key, Value> TopMap<Key, Value>
where
    Key: Copy + Ord,
    isize: From<Key>,
{
    pub fn reserve(&mut self, additional: usize) {
        self.top.reserve(additional)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &Value)> {
        self.top
            .iter()
            .filter_map(|entry| entry.as_ref().map(|(key, value)| (*key, value)))
            .chain(self.rest.iter().map(|(key, value)| (*key, value)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Key, &mut Value)> {
        self.top
            .iter_mut()
            .filter_map(|entry| entry.as_mut().map(|(key, value)| (*key, value)))
            .chain(self.rest.iter_mut().map(|(key, value)| (*key, value)))
    }

    fn index(&self, key: Key) -> Index {
        let index = if let &Some((first_key, _)) = &self.first {
            isize::from(key) - isize::from(first_key)
        } else {
            0
        };

        if let Some(index) = positive(index) {
            if index >= self.top_count {
                Index::Rest
            } else if index > 0 {
                Index::InsideTop { index: index - 1 }
            } else {
                Index::First
            }
        } else {
            Index::AboveTop {
                distance: -index as usize,
            }
        }
    }

    fn remove_end(&mut self, first: bool) -> Option<Value> {
        let value = if first {
            let (_, value) = mem::replace(&mut self.first, self.top.pop_front()?)?;

            while let Some(None) = self.top.front() {
                self.top.pop_front();
            }

            value
        } else {
            let (_, value) = self.top.pop_back()??;
            value
        };

        let first_key = if let &Some((first_key, _)) = &self.first {
            Some(first_key)
        } else if let Some((&rest_key, _)) = self.rest.iter().next() {
            assert!(self.top.is_empty());

            let rest_value = self.rest.remove(&rest_key).unwrap();
            self.first = Some((rest_key, rest_value));
            Some(rest_key)
        } else {
            None
        };

        if let Some(first_key) = first_key {
            while let Some((&key, _)) = self.rest.iter().next() {
                let index = positive(isize::from(key) - isize::from(first_key))
                    .expect("everything in the rest map should have an index higher than everything in the top vec")
                    .checked_sub(1)
                    .expect("item with index=0 found in rest but we thought we already removed that");

                if index >= self.top_count {
                    break;
                }

                let value = self.rest.remove(&key).unwrap();
                *ensure_index(&mut self.top, index) = Some((key, value));
            }
        }

        Some(value)
    }

    pub fn entry(&mut self, key: Key) -> Entry<Key, Value> {
        match self.index(key) {
            Index::AboveTop { distance } => Entry::AboveTop {
                key,
                map: self,
                distance,
            },

            Index::First => Entry::Vec {
                key,
                entry: &mut self.first,
            },

            Index::InsideTop { index } => Entry::Vec {
                key,
                entry: ensure_index(&mut self.top, index),
            },

            Index::Rest => Entry::BTreeMap(self.rest.entry(key)),
        }
    }

    pub fn get(&self, key: Key) -> Option<&Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,
            Index::First => Some(&self.first.as_ref()?.1),
            Index::InsideTop { index } => Some(&self.top.get(index)?.as_ref()?.1),
            Index::Rest => self.rest.get(&key),
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,
            Index::First => Some(&mut self.first.as_mut()?.1),
            Index::InsideTop { index } => Some(&mut self.top.get_mut(index)?.as_mut()?.1),
            Index::Rest => self.rest.get_mut(&key),
        }
    }

    pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        self.entry(key).insert(value)
    }

    pub fn remove(&mut self, key: Key) -> Option<Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,

            Index::First => self.remove_end(true),

            Index::InsideTop { index } => {
                let value = if index == self.top_count - 1 {
                    self.remove_end(false)?
                } else {
                    let (_, value) = mem::replace(self.top.get_mut(index)?, None)?;
                    value
                };

                Some(value)
            }

            Index::Rest => self.rest.remove(&key),
        }
    }
}

impl<Key, Value> ops::Index<Key> for TopMap<Key, Value>
where
    Key: Copy + Ord + fmt::Debug,
    isize: From<Key>,
{
    type Output = Value;

    fn index(&self, index: Key) -> &Value {
        self.get(index)
            .unwrap_or_else(|| panic!("no item with key {:?}", index))
    }
}

impl<Key, Value> ops::IndexMut<Key> for TopMap<Key, Value>
where
    Key: Copy + Ord + fmt::Debug,
    isize: From<Key>,
{
    fn index_mut(&mut self, index: Key) -> &mut Value {
        self.get_mut(index)
            .unwrap_or_else(|| panic!("no item with key {:?}", index))
    }
}

impl<Key, Value> Extend<(Key, Value)> for TopMap<Key, Value>
where
    Key: Copy + Ord,
    isize: From<Key>,
{
    fn extend<T: IntoIterator<Item = (Key, Value)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TopMap;

    static ITEMS: &[(isize, &'static str)] = &[
        (100, "a1"),
        (101, "a2"),
        (200, "b1"),
        (201, "b2"),
        (300, "c1"),
        (301, "c2"),
    ];

    fn lens<Key, Value>(m: &TopMap<Key, Value>) -> [usize; 3] {
        [
            m.len(),
            (if m.first.is_some() { 1 } else { 0 } + m.top.iter().filter(|&entry| entry.is_some()).count()),
            m.rest.len(),
        ]
    }

    #[test]
    fn extend() {
        let mut m = TopMap::new(10);
        m.extend(ITEMS.iter().cloned());
        assert_eq!([6, 2, 4], lens(&m));

        let items = m.iter()
            .map(|(key, &value)| (key, value))
            .collect::<Vec<(isize, &'static str)>>();

        assert_eq!(ITEMS, &items[..]);
    }

    #[test]
    fn insert() {
        let mut m = TopMap::new(10);
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
        let mut m = TopMap::new(10);
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
        let mut m = TopMap::new(100);
        m.extend((0..1000).map(|n| (n as isize, n)));
        assert_eq!([1000, 100, 900], lens(&m));

        let index = -1;
        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 100, 901], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 100, 900], lens(&m));

        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 100, 901], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 100, 900], lens(&m));
    }

    #[test]
    fn insert_remove_existing_m3() {
        let mut m = TopMap::new(100);
        m.extend((0..1000).map(|n| (n as isize, n)));
        assert_eq!([1000, 100, 900], lens(&m));

        let index = -3;
        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 98, 903], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 100, 900], lens(&m));

        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 98, 903], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 100, 900], lens(&m));
    }

    #[test]
    fn insert_remove_existing_m999() {
        let mut m = TopMap::new(100);
        m.extend((0..1000).map(|n| (n as isize, n)));
        assert_eq!([1000, 100, 900], lens(&m));

        let index = -999;
        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 1, 1000], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 100, 900], lens(&m));

        assert_eq!(None, m.insert(index, index));
        assert_eq!([1001, 1, 1000], lens(&m));

        assert_eq!(Some(index), m.remove(index));
        assert_eq!([1000, 100, 900], lens(&m));
    }
}
