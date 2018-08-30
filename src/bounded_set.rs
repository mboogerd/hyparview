use std::collections::HashSet;
use std::hash::Hash;

pub struct SetWithCapacity<E> {
    pub capacity: usize,
    wraps: HashSet<E>,
}

impl<E: Eq + Hash> SetWithCapacity<E> {
    pub fn new(capacity: usize) -> SetWithCapacity<E> {
        SetWithCapacity::init(capacity, HashSet::new())
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

// impl<E> Iterator for SetWithCapacity<E> {
//     type Item=E;
//     fn next(&mut self) -> Option<E> {
//         self.wraps.iter().next()
//     }
// }

#[cfg(test)]
mod test {

    use super::BoundedSet;
    use super::SetWithCapacity;
    use std::collections::HashSet;

    #[test]
    #[should_panic]
    fn init_should_respect_capacity() {
        let mut data: HashSet<u32> = HashSet::new();
        data.insert(1);
        // should panic!
        SetWithCapacity::init(0, data);
    }

    #[test]
    fn init_should_pass_initial_state() {
        let mut data: HashSet<u32> = HashSet::new();
        data.insert(1);
        let set = SetWithCapacity::init(1, data);
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
}

pub trait BoundedSet<E> {
    fn is_full(&self) -> bool;
    //     fn contains(elem: E) -> bool;
    //     fn add(elem: E) -> Self;
    //     fn remove(elem: E) -> Self;
    //     fn iter() -> Iter<E>;
    //     fn sample_one() -> (Option<E>, Self);
    //     fn sample() -> Self;
    //     fn bounded_union(to_merge: Self, drop_priority: HashSet<E>) -> Self;
}

impl<E: Eq + Hash> BoundedSet<E> for SetWithCapacity<E> {
    fn is_full(&self) -> bool {
        self.capacity == self.wraps.len()
    }
}

// pub trait BoundedOps<E>
