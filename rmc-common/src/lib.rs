use std::ops::{Add, Mul, Neg, Sub};
use vek::num_traits::{One, Zero};

pub mod game;
pub mod world;
pub use game::Game;
pub mod input;
pub mod light;
pub mod physics;

mod blend;
mod camera;
pub use blend::{Blend, DiscreteBlend};
pub use camera::Camera;

pub trait Apply: Sized {
    /// Apply a function to this value and return the (possibly) modified value.
    fn apply<F: FnOnce(&mut Self)>(mut self, block: F) -> Self {
        block(&mut self);
        self
    }
}

impl<T> Apply for T {}

pub struct LookBack<T> {
    pub prev: T,
    pub curr: T,
}

impl<T> LookBack<T> {
    pub fn new(prev: T, curr: T) -> Self {
        LookBack { prev, curr }
    }

    pub fn push(&mut self, new: T) {
        self.prev = std::mem::replace(&mut self.curr, new);
    }
}

impl<T: Clone> LookBack<T> {
    pub fn new_identical(curr: T) -> Self {
        Self::new(curr.clone(), curr)
    }

    pub fn push_from(&mut self, f: impl FnOnce(&T, &mut T)) {
        let mut new = self.curr.clone();
        f(&self.prev, &mut new);
        self.push(new);
    }
}

pub fn lerp<T, U: Copy>(a: T, b: T, alpha: U) -> T
where
    T: Mul<U, Output = T> + Add<T, Output = T>,
    f32: Sub<U, Output = U>,
{
    a * (1.0 - alpha) + b * alpha
}

#[test]
fn test_look_back() {
    let mut a = LookBack::new_identical(0.0);
    a.push(1.0);

    assert_eq!(a.prev, 0.0);
    assert_eq!(a.curr, 1.0);

    a.push(2.0);

    assert_eq!(a.prev, 1.0);
    assert_eq!(a.curr, 2.0);

    a.push_from(|&prev, val| {
        assert_eq!(prev, 1.0);
        assert_eq!(*val, 2.0);

        *val = 3.0;
    });

    assert_eq!(a.prev, 2.0);
    assert_eq!(a.curr, 3.0);
}

pub trait SignNum2 {
    fn signum2(&self) -> Self;
}

impl<T: Zero + One + Neg<Output = T> + PartialOrd> SignNum2 for T {
    fn signum2(&self) -> Self {
        if self > &Self::zero() {
            Self::one()
        } else if self < &Self::zero() {
            -Self::one()
        } else {
            Self::zero()
        }
    }
}
