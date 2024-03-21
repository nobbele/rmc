use vek::{Mat4, Quaternion, Vec3, Wrap};

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub position: Vec3<f32>,
    pub pitch: f32,
    pub yaw: f32,
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
        Quaternion::rotation_y(-self.yaw) * -Vec3::unit_z()
    }

    pub fn right(&self) -> Vec3<f32> {
        Quaternion::rotation_y(-self.yaw) * Vec3::unit_x()
    }

    pub fn to_matrix(&self) -> Mat4<f32> {
        Mat4::<f32>::identity()
            .translated_3d(-self.position)
            .rotated_y(self.yaw)
            .rotated_x(self.pitch)
    }

    pub fn rotate_horizontal(&mut self, v: f32) {
        self.yaw = (self.yaw + v).wrapped_2pi();
    }

    pub fn rotate_vertical(&mut self, v: f32) {
        self.pitch = (self.pitch + v).clamp(-std::f32::consts::PI / 2., std::f32::consts::PI / 2.);
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
