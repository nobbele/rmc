use ndarray::Array3;
use std::{
    collections::{HashSet, VecDeque},
    ops::{Add, Mul, Neg, Sub},
    rc::Rc,
};
use vek::{
    num_traits::{One, Zero},
    Vec3,
};

pub mod world;

mod camera;
pub use camera::Camera;
pub mod game;
pub use game::Game;
pub mod input;
pub mod physics;

pub trait Blend {
    fn blend(&self, other: &Self, alpha: f32) -> Self;
}

impl Blend for f32 {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        *self * (1.0 - alpha) + *other * alpha
    }
}

impl Blend for i32 {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        (*self as f32).blend(&(*other as f32), alpha) as _
    }
}

impl Blend for usize {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        (*self as f32).blend(&(*other as f32), alpha) as _
    }
}

impl<T: Blend + Clone> Blend for Vec3<T> {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        self.clone()
            .zip(other.clone())
            .map(|(a, b)| a.blend(&b, alpha))
    }
}

impl<T: Blend + PartialEq> Blend for Rc<T> {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        if self == other {
            self.clone()
        } else {
            Rc::new((&**self).blend(&**other, alpha))
        }
    }
}

impl<T: Blend + Default + Clone> Blend for Array3<T> {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        assert_eq!(self.shape(), other.shape());

        let shape = self.shape();
        let mut result = Array3::default((shape[0], shape[1], shape[2]));
        for (idx, el) in self.indexed_iter() {
            if let Some(new_el) = Some(el.clone()).blend(&other.get(idx).cloned(), alpha) {
                result[idx] = new_el;
            }
        }
        result
    }
}

impl<T: Blend + Clone> Blend for Option<T> {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        if let (Some(a), Some(b)) = (self, other) {
            return Some(a.blend(b, alpha));
        }

        if alpha < 0.5 {
            self.clone()
        } else {
            other.clone()
        }
    }
}

impl<T: DiscreteBlend + Clone> Blend for T {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        if alpha < 0.5 {
            self.clone()
        } else {
            other.clone()
        }
    }
}

pub trait DiscreteBlend {}

impl DiscreteBlend for bool {}
impl<T> DiscreteBlend for Vec<T> {}
impl<T> DiscreteBlend for VecDeque<T> {}
impl<T> DiscreteBlend for HashSet<T> {}

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
