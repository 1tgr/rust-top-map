#![deny(warnings)]
#![deny(unused_extern_crates)]

use std::collections::{BTreeMap, VecDeque};
use std::collections::btree_map;
use std::fmt;
use std::mem;
use std::ops;

pub struct TopMap<Key, Value> {
    top_count: usize,
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
    InsideTop { index: usize },
    Rest,
}

pub enum Entry<'a, Key: 'a, Value: 'a> {
    Vec(Key, &'a mut Option<(Key, Value)>),
    BTreeMap(btree_map::Entry<'a, Key, Value>),
}

impl<'a, Key, Value> Entry<'a, Key, Value>
where
    Key: Ord,
{
    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::Vec(key, entry) => &mut entry.get_or_insert((key, default)).1,
            Entry::BTreeMap(entry) => entry.or_insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> Value>(self, default: F) -> &'a mut Value {
        match self {
            Entry::Vec(key, entry) => &mut entry.get_or_insert_with(|| (key, default())).1,
            Entry::BTreeMap(entry) => entry.or_insert_with(default),
        }
    }
}

impl<Key, Value> TopMap<Key, Value> {
    pub fn len(&self) -> usize {
        self.top.len() + self.rest.len()
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
        let index = if let Some(ref min_entry) = self.top.front() {
            let &(min_key, _) = min_entry.as_ref().expect("top entry should be filled");
            isize::from(key) - isize::from(min_key)
        } else {
            0
        };

        if let Some(index) = positive(index) {
            if index >= self.top_count {
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

    pub fn entry(&mut self, key: Key) -> Entry<Key, Value> {
        match self.index(key) {
            Index::AboveTop { distance } => {
                let new_count = positive(self.top_count as isize - distance as isize).unwrap_or(0);
                for entry in self.top.drain(new_count..) {
                    if let Some((key, value)) = entry {
                        self.rest.insert(key, value);
                    }
                }

                assert_eq!(new_count, self.top.len());
                Entry::Vec(key, &mut self.top[0])
            }

            Index::InsideTop { index } => {
                while self.top.len() <= index {
                    self.top.push_back(None);
                }

                Entry::Vec(key, &mut self.top[index])
            }

            Index::Rest => Entry::BTreeMap(self.rest.entry(key)),
        }
    }

    pub fn get(&self, key: Key) -> Option<&Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,
            Index::InsideTop { index } => Some(&self.top.get(index)?.as_ref()?.1),
            Index::Rest => self.rest.get(&key),
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,
            Index::InsideTop { index } => Some(&mut self.top.get_mut(index)?.as_mut()?.1),
            Index::Rest => self.rest.get_mut(&key),
        }
    }

    pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
        match self.index(key) {
            Index::AboveTop { distance } => {
                let new_count = positive(self.top_count as isize - distance as isize).unwrap_or(0);
                for entry in self.top.drain(new_count..) {
                    if let Some((key, value)) = entry {
                        self.rest.insert(key, value);
                    }
                }

                assert!(self.top.len() <= new_count);
                self.top.push_front(Some((key, value)));
                None
            }

            Index::InsideTop { index } => loop {
                if let Some(entry) = self.top.get_mut(index) {
                    return mem::replace(entry, Some((key, value))).map(|(_, value)| value);
                }

                self.top.push_back(None);
            },

            Index::Rest => {
                self.rest.insert(key, value);
                None
            }
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<Value> {
        match self.index(key) {
            Index::AboveTop { distance: _ } => None,

            Index::InsideTop { index } => {
                let (_, value) = mem::replace(self.top.get_mut(index)?, None)?;

                if index == 0 || index == self.top.len() - 1 {
                    self.top.remove(index);

                    let min_top_key = if let Some(&Some((min_top_key, _))) = self.top.front() {
                        Some(min_top_key)
                    } else if let Some((&rest_key, _)) = self.rest.iter().next() {
                        let rest_value = self.rest.remove(&rest_key).unwrap();
                        self.top.push_back(Some((rest_key, rest_value)));
                        Some(rest_key)
                    } else {
                        None
                    };

                    if let Some(min_top_key) = min_top_key {
                        while let Some((&rest_key, _)) = self.rest.iter().next() {
                            let rest_index = positive(isize::from(rest_key) - isize::from(min_top_key)).expect(
                                "everything in the rest map should have an index higher than everything in the top vec",
                            );

                            if rest_index >= self.top_count {
                                break;
                            }

                            while self.top.len() <= rest_index {
                                self.top.push_back(None);
                            }

                            let rest_value = self.rest.remove(&rest_key).unwrap();
                            self.top[rest_index] = Some((rest_key, rest_value));
                        }
                    }
                }

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
        [m.len(), m.top.len(), m.rest.len()]
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
}
