use std::ops;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NodePin<T> {
    initial: T,
    value: Option<T>,
}

impl<T> NodePin<T> {
    pub fn new(initial: T) -> Self {
        Self { initial, value: None }
    }

    pub fn set(&mut self, value: T) {
        self.value = Some(value);
    }

    pub fn set_initial(&mut self, initial: T) {
        self.initial = initial;
    }

    pub fn reset(&mut self) {
        self.value = None;
    }

    pub fn as_ref(&self) -> &T {
        self.value.as_ref().unwrap_or(&self.initial)
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.value.as_mut().unwrap_or(&mut self.initial)
    }
}

impl ops::Deref for NodePin<f64> {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Copy> NodePin<T> {
    pub fn get(&self) -> T {
        self.value.unwrap_or(self.initial)
    }
}
