use rustc_hash::{FxBuildHasher, FxHashSet};
use std::io::{Error, ErrorKind, Result};

#[derive(Debug)]
struct Entry {
    deadline: u64,
    token: usize,
}
pub struct TimerHeap {
    tokens: FxHashSet<usize>,
    entries: Vec<Entry>,
}

impl TimerHeap {
    pub fn new(capacity: usize) -> Result<Self> {
        match std::panic::catch_unwind(|| TimerHeap {
            tokens: FxHashSet::<usize>::with_capacity_and_hasher(capacity, FxBuildHasher),
            entries: Vec::<Entry>::with_capacity(capacity),
        }) {
            Ok(th) => Ok(th),
            Err(_) => Err(Error::from(ErrorKind::OutOfMemory)),
        }
    }
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn push(&mut self, token: usize, deadline: u64) -> Result<()> {
        if self.entries.try_reserve(1).is_err() || self.tokens.try_reserve(1).is_err() {
            Err(Error::from(ErrorKind::OutOfMemory))
        } else if !self.tokens.insert(token) {
            Err(Error::from(ErrorKind::AlreadyExists))
        } else {
            let index = self.entries.len();
            self.entries.push(Entry { deadline, token });
            self.restore_up(index);
            Ok(())
        }
    }

    pub fn pop(&mut self) -> Option<usize> {
        // Clean up removed elements "tombstones"
        if !self.tokens.is_empty() {
            // Remove the first element
            let new_len = self.entries.len() - 1;
            let token = self.entries[0].token;
            self.entries.swap(0, new_len);
            self.entries.truncate(new_len);
            self.restore_down(0);
            self.tokens.remove(&token);
            // Ensure that self.entries[0].token is always valid (when self.len() > 0)
            if !self.entries.is_empty() && !self.tokens.contains(&self.entries[0].token) {
                self.discard_head_tombstones();
            }
            Some(token)
        } else {
            None
        }
    }

    pub fn remove(&mut self, token: usize) -> bool {
        // println!("remove({}) => ({:?})", token, self.entries);
        let was_present = self.tokens.remove(&token);
        if was_present && self.entries[0].token == token {
            // Ensure that self.entries[0].token is always valid (when self.len() > 0)
            self.discard_head_tombstones();
        }
        // println!(
        //     " => remove({}) => {was_present} ({:?} --- {:?})",
        //     token, self.tokens, self.entries
        // );
        was_present
    }

    pub fn next(&self) -> Option<usize> {
        if !self.tokens.is_empty() {
            Some(self.entries[0].token)
        } else {
            None
        }
    }

    pub fn next_deadline(&self, current_deadline: u64) -> u64 {
        // println!(
        //     "next_deadline: token.len()={}, entries.len()={})",
        //     self.tokens.len(),
        //     self.entries.len(),
        // );
        if !self.tokens.is_empty() && self.entries[0].deadline < current_deadline {
            self.entries[0].deadline
        } else {
            current_deadline
        }
    }

    #[inline]
    fn parent(index: usize) -> usize {
        index.wrapping_sub(1) >> 2
    }
    #[inline]
    fn child(parent: usize, child_index: usize) -> usize {
        (parent << 2) + 1 + child_index
    }

    fn discard_head_tombstones(&mut self) {
        // println!(
        //     "Tombstone: token.len()={}, entries.len()={}, new_len={})",
        //     self.tokens.len(),
        //     self.entries.len(),
        //     new_len
        // );
        // Clean up removed elements "tombstones"
        while !self.entries.is_empty() && !self.tokens.contains(&self.entries[0].token) {
            let new_len = self.entries.len().saturating_sub(1);
            // println!(" 1 Tombstone {:?}", self.entries);
            self.entries.swap(0, new_len);
            // println!(" 2 Tombstone {:?}", self.entries);
            self.entries.truncate(new_len);
            // println!(" 3 Tombstone {:?}", self.entries);
            self.restore_down(0);
            // println!(" 4 Tombstone {:?}", self.entries);
        }
    }

    fn restore_up(&mut self, index: usize) {
        let mut index = index;
        let mut parent = TimerHeap::parent(index);
        while index > 0 && self.entries[index].deadline < self.entries[parent].deadline {
            self.entries.swap(index, parent);
            index = parent;
            parent = TimerHeap::parent(index);
        }
    }
    fn restore_down(&mut self, index: usize) {
        let mut index = index;
        let len = self.entries.len();
        while index < len {
            let mut min_index = usize::MAX;
            let mut min_deadline = self.entries[index].deadline;

            let child = TimerHeap::child(index, 0);
            if child < len && self.entries[child].deadline < min_deadline {
                min_deadline = self.entries[child].deadline;
                min_index = child;
            }
            let child = TimerHeap::child(index, 1);
            if child < len && self.entries[child].deadline < min_deadline {
                min_deadline = self.entries[child].deadline;
                min_index = child;
            }
            let child = TimerHeap::child(index, 2);
            if child < len && self.entries[child].deadline < min_deadline {
                min_deadline = self.entries[child].deadline;
                min_index = child;
            }
            let child = TimerHeap::child(index, 3);
            if child < len && self.entries[child].deadline < min_deadline {
                min_index = child;
            }

            if min_index == usize::MAX {
                return;
            }
            self.entries.swap(index, min_index);
            index = min_index;
        }
    }
}

#[cfg(test)]
mod test {
    use rand::seq::SliceRandom;

    use super::*;

    #[test]
    fn test_simple() {
        let mut th = TimerHeap::new(32).unwrap();
        assert_eq!(th.len(), 0);
        assert_eq!(th.next_deadline(u64::MAX), u64::MAX);
        assert!(th.push(5 as _, 5 as _).is_ok());
        assert!(th
            .push(5 as _, 99 as _)
            .is_err_and(|e| e.kind() == ErrorKind::AlreadyExists));
        assert_eq!(th.len(), 1);
        assert_eq!(th.next_deadline(u64::MAX), 5u64);
        assert!(th.push(18usize, 18u64).is_ok());
        assert_eq!(th.len(), 2);
        assert_eq!(th.next_deadline(u64::MAX), 5u64);
        assert!(th.remove(5usize));
        assert!(!th.remove(5usize));
        assert_eq!(th.len(), 1);
        assert_eq!(th.next_deadline(u64::MAX), 18u64);
        assert!(th.remove(18usize));
        assert!(!th.remove(18usize));
        assert_eq!(th.len(), 0);
        assert_eq!(th.next_deadline(u64::MAX), u64::MAX);
    }
    #[test]
    fn test_random() {
        let mut th = TimerHeap::new(32).unwrap();
        let mut rng = rand::thread_rng();
        let mut data: Vec<u64> = (0..65536).collect();
        data.shuffle(&mut rng);
        for v in data.iter() {
            assert!(th.push(*v as _, *v).is_ok());
        }
        data.sort();
        for v in data.iter() {
            if (v & 1u64) != 0 {
                let popped_token = th.pop();
                assert!(popped_token.is_some());
                assert_eq!(popped_token.unwrap(), *v as usize);
            } else {
                let popped_token = th.next();
                assert!(popped_token.is_some());
                let popped_token = popped_token.unwrap();
                assert_eq!(popped_token, *v as usize);
                assert!(th.remove(popped_token as usize));
            }
        }
    }
}
