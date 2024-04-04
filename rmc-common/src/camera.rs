use crate::Blend;
use std::f32::consts::TAU;
use vek::{Mat4, Quaternion, Vec3, Wrap};

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
/// 0 to tau
pub struct Angle(pub f32);

impl Blend for Angle {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        // This is to handle the value as a ring/modulo rather than standard line.
        // i.e to get to 0 it might be closer to blend towards tau and vice versa.

        // 0 ----- pi ---- tau
        //               ^ if self is here
        //   ^ and other is here
        // then the result should blend towards tau wrapped, not 0
        if ((other.0 + TAU) - self.0).abs() < (other.0 - self.0).abs() {
            Angle(self.0.blend(&(other.0 + TAU), alpha).wrapped_2pi())
        }
        // 0 ----- pi ---- tau
        //    ^ if self is here
        //              ^ and other is here
        // then the result should blend towards 0, not tau
        else if (other.0 - (self.0 + TAU)).abs() < (other.0 - self.0).abs() {
            Angle((self.0 + TAU).blend(&other.0, alpha).wrapped_2pi())
        }
        // In every other case, blend normally
        else {
            Angle(self.0.blend(&other.0, alpha))
        }
    }
}

impl Angle {
    pub fn sin(self) -> f32 {
        self.0.sin()
    }

    pub fn cos(self) -> f32 {
        self.0.cos()
    }
}

impl std::ops::Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(self.0.neg().wrapped_2pi())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub position: Vec3<f32>,
    pub pitch: Angle,
    pub yaw: Angle,
}

impl Camera {
    pub fn look_at(&self) -> Vec3<f32> {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            -self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn forward(&self) -> Vec3<f32> {
        Quaternion::rotation_y(-self.yaw.0) * -Vec3::unit_z()
    }

    pub fn right(&self) -> Vec3<f32> {
        Quaternion::rotation_y(-self.yaw.0) * Vec3::unit_x()
    }

    pub fn to_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::identity()
            .translated_3d(-self.position)
            .rotated_y(self.yaw.0)
            .rotated_x(self.pitch.0)
    }

    pub fn rotate_horizontal(&mut self, v: f32) {
        self.yaw.0 = (self.yaw.0 + v).wrapped_2pi();
    }

    pub fn rotate_vertical(&mut self, v: f32) {
        self.pitch.0 =
            (self.pitch.0 + v).clamp(-std::f32::consts::PI / 2., std::f32::consts::PI / 2.);
    }

    pub fn move_forward(&mut self, v: f32) {
        self.position += v * self.forward();
    }

    pub fn move_right(&mut self, v: f32) {
        self.position += v * self.right();
    }

    pub fn move_up(&mut self, v: f32) {
        self.position.y += v;
    }
}

impl Blend for Camera {
    fn blend(&self, other: &Self, alpha: f32) -> Self {
        Self {
            position: self.position.blend(&other.position, alpha),
            pitch: self.pitch.blend(&other.pitch, alpha),
            yaw: self.yaw.blend(&other.yaw, alpha),
        }
    }
}
