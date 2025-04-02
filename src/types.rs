use serde::{Deserialize, Serialize};

pub type Color = egui::Color32;
pub type Vector3 = nalgebra::Vector3<f64>;
pub type Point3 = Vector3;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Basis {
    pub u: Vector3,
    pub v: Vector3,
    pub w: Vector3,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vector3,
}

impl Ray {
    pub fn new(origin: Point3, direction: Vector3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    pub fn at(&self, t: f64) -> Point3 {
        self.origin + t * self.direction
    }
}

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

impl<T: Copy> NodePin<T> {
    pub fn get(&self) -> T {
        self.value.unwrap_or(self.initial)
    }
}
