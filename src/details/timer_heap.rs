use rustc_hash::FxHashSet;
use std::{
    io::{Error, ErrorKind, Result},
    u64, usize,
};

struct Entry {
    deadline: u64,
    token: usize,
}
pub struct TimerHeap {
    tokens: FxHashSet<usize>,
    entries: Vec<Entry>,
}

impl TimerHeap {
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn push(&mut self, deadline: u64, token: usize) -> Result<()> {
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
        self.discard_head_tombstones();
        if self.tokens.len() > 0 {
            // Remove the first element
            let new_len = self.entries.len() - 1;
            let token = self.entries[0].token;
            self.entries.swap(0, new_len);
            self.restore_down(0);
            self.entries.truncate(new_len);
            self.tokens.remove(&token);
            Some(token)
        } else {
            None
        }
    }
    pub fn remove(&mut self, token: usize) -> bool {
        let was_present = self.tokens.remove(&token);
        if was_present && self.entries[0].token == token {
            // Clean up removed elements "tombstones"
            self.discard_head_tombstones();
        }
        was_present
    }

    #[inline]
    fn parent(index: usize) -> usize {
        (index - 1) >> 2
    }
    #[inline]
    fn child(parent: usize, child_index: usize) -> usize {
        (index << 2) + 1 + child_index
    }

    fn discard_head_tombstones(&mut self) {
        let mut new_len = self.entries.len().saturating_sub(1);
        // Clean up removed elements "tombstones"
        while new_len > 0 && !self.tokens.contains(&self.entries[0].token) {
            new_len -= 1;
            self.entries.swap(0, new_len);
            self.restore_down(0);
            self.entries.truncate(new_len);
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
        loop {
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
