extern crate rand;

use std::collections::HashSet;
use std::hash::Hash;
use std::mem;

#[derive(Eq, PartialEq, Debug)]
pub struct SetWithCapacity<E: Hash + Eq + PartialEq> {
    pub capacity: usize,
    wraps: HashSet<E>,
}

impl<E: Eq + PartialEq + Hash> SetWithCapacity<E> {
    pub fn new(capacity: usize) -> SetWithCapacity<E> {
        SetWithCapacity::init(capacity, HashSet::with_capacity(capacity))
    }

    pub fn single(capacity: usize, singleton: E) -> SetWithCapacity<E> {
        let mut set: HashSet<E> = HashSet::with_capacity(capacity);
        set.insert(singleton);
        SetWithCapacity::init(capacity, set)
    }

    pub fn init(capacity: usize, wraps: HashSet<E>) -> SetWithCapacity<E> {
        if wraps.len() > capacity {
            panic!("Capacity of this new SetWithCapacity exceeds the size of the initial state")
        }

        SetWithCapacity {
            capacity: capacity,
            wraps: wraps,
        }
    }
}

#[cfg(test)]
mod test {

    use super::BoundedSet;
    use super::SetWithCapacity;
    use std::collections::HashSet;

    const TEST_ELEM: u32 = 1;

    fn singleton_set() -> HashSet<u32> {
        let mut data: HashSet<u32> = HashSet::new();
        data.insert(TEST_ELEM);
        data
    }

    #[test]
    #[should_panic]
    fn init_should_respect_capacity() {
        // should panic!
        SetWithCapacity::single(0, TEST_ELEM);
    }

    #[test]
    fn init_should_pass_initial_state() {
        let set = SetWithCapacity::single(1, TEST_ELEM);
        assert_eq!(set.wraps.len(), 1)
    }

    #[test]
    fn is_full_when_capacity_is_size() {
        // Capacity == size
        let set: SetWithCapacity<u32> = SetWithCapacity::new(0);
        assert!(BoundedSet::is_full(&set));
        // Capacity > size
        let set: SetWithCapacity<u32> = SetWithCapacity::new(1);
        assert!(!set.is_full())
    }

    #[test]
    fn empty_random_element() {
        let set: SetWithCapacity<u32> = SetWithCapacity::new(1);
        assert!(set.sample_one().is_none());
    }

    #[test]
    fn singleton_random_element() {
        let set: SetWithCapacity<u32> = SetWithCapacity::single(1, TEST_ELEM);
        assert!(set.sample_one().unwrap() == &TEST_ELEM);
    }

    #[test]
    fn empty_sample() {
        let set: SetWithCapacity<u32> = SetWithCapacity::new(1);
        assert!(set.sample(10).is_empty());
    }

    #[test]
    fn bounded_sample() {
        let set: SetWithCapacity<u32> = SetWithCapacity::single(1, TEST_ELEM);
        assert!(set.sample(0).is_empty());
    }

    #[test]
    fn max_sample() {
        let set: SetWithCapacity<u32> = SetWithCapacity::single(1, TEST_ELEM);
        let sample = set.sample(10);
        assert_eq!(sample.len(), 1);
        assert!(sample.contains(&TEST_ELEM));
    }

    #[test]
    fn contains() {
        let set = SetWithCapacity::init(1, singleton_set());
        assert!(set.contains(&TEST_ELEM));
        assert!(!set.contains(&1337));
    }

    #[test]
    fn insert() {
        let mut set1: SetWithCapacity<u32> = SetWithCapacity::new(1);
        let set2: SetWithCapacity<u32> = SetWithCapacity::single(1, TEST_ELEM);

        assert!(set1.insert(TEST_ELEM));
        assert_eq!(set1, set2);
    }

    #[test]
    fn insert_respects_capacity() {
        let mut set: SetWithCapacity<u32> = SetWithCapacity::new(0);
        assert!(!set.insert(TEST_ELEM));
    }

    #[test]
    fn remove() {
        let mut set: SetWithCapacity<u32> = SetWithCapacity::single(1, TEST_ELEM);
        assert!(!set.remove(&1337));
        assert_eq!(set.wraps.len(), 1);

        assert!(set.remove(&TEST_ELEM));
        assert_eq!(set.wraps.len(), 0);
    }

    #[test]
    fn bounded_union() {
        let mut set1: SetWithCapacity<u32> = SetWithCapacity::single(10, TEST_ELEM);
        let mut set2: HashSet<u32> = HashSet::new();
        set2.insert(2);
        set1.bounded_union(&set2, &HashSet::new());

        assert!(set1.contains(&TEST_ELEM));
        assert!(set1.contains(&2));
    }

    #[test]
    fn bounded_union_drop_priority() {
        let mut set: SetWithCapacity<u32> = SetWithCapacity::single(1, TEST_ELEM);
        let mut to_merge: HashSet<u32> = HashSet::new();
        to_merge.insert(2);
        let mut drop_prio: HashSet<u32> = HashSet::new();
        drop_prio.insert(TEST_ELEM);

        set.bounded_union(&to_merge, &drop_prio);
        assert_eq!(set.wraps.len(), 1);
        assert!(set.contains(&2));
    }
}

pub trait BoundedSet<E> {
    fn is_full(&self) -> bool;
    fn sample_one(&self) -> Option<&E>;
    fn sample(&self, max_size: usize) -> HashSet<&E>;
    fn contains(&self, elem: &E) -> bool;
    fn insert(&mut self, elem: E) -> bool;
    fn remove(&mut self, elem: &E) -> bool;
    //     fn iter(&self) -> Iter<E>;
    fn bounded_union(&mut self, to_merge: &HashSet<E>, drop_priority: &HashSet<E>);
    fn len(&self) -> usize;
}

impl<E: Eq + Hash + Clone> BoundedSet<E> for SetWithCapacity<E> {
    fn is_full(&self) -> bool {
        self.capacity == self.wraps.len()
    }

    fn sample_one(&self) -> Option<&E> {
        // Sets are somewhat randomly distributed
        // FIXME: a bias will exist if the same hash is used consistently
        self.wraps.iter().next()
    }

    fn sample(&self, max_size: usize) -> HashSet<&E> {
        self.wraps.iter().take(max_size).collect::<HashSet<&E>>()
    }

    fn contains(&self, elem: &E) -> bool {
        self.wraps.contains(elem)
    }

    fn insert(&mut self, elem: E) -> bool {
        if self.capacity > self.wraps.len() {
            self.wraps.insert(elem)
        } else {
            false
        }
    }

    fn remove(&mut self, elem: &E) -> bool {
        self.wraps.remove(elem)
    }

    fn len(&self) -> usize {
        self.wraps.len()
    }

    fn bounded_union(&mut self, to_merge: &HashSet<E>, drop_priority: &HashSet<E>) {
        let mut replacement: HashSet<E> = HashSet::new();
        mem::swap(&mut self.wraps, &mut replacement);
        let iter = to_merge
            .iter()
            .chain(replacement.difference(drop_priority))
            .chain(drop_priority.intersection(&replacement))
            .cloned();

        for e in iter {
            if self.capacity > self.len() {
                self.wraps.insert(e);
            } else {
                return;
            }
        }
    }
}
