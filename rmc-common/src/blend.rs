use std::{
    collections::{HashSet, VecDeque},
    rc::Rc,
};

use ndarray::Array3;
use vek::Vec3;

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
