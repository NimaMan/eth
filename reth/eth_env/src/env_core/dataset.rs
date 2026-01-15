use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledObservation<O, T> {
    pub observation: O,
    pub target: T,
}

pub struct ObservationDataset<O, T> {
    capacity: usize,
    items: VecDeque<LabeledObservation<O, T>>,
}

impl<O, T> ObservationDataset<O, T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, item: LabeledObservation<O, T>) {
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn iter(&self) -> impl Iterator<Item = &LabeledObservation<O, T>> {
        self.items.iter()
    }
}
