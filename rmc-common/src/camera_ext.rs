use vek::Vec3;

use crate::{world::CHUNK_SIZE, Camera};

pub trait CameraExt {
    fn is_chunk_in_view(&self, chunk_coord: Vec3<i32>) -> bool;
}

impl CameraExt for Camera {
    fn is_chunk_in_view(&self, chunk_coord: Vec3<i32>) -> bool {
        let chunk_corner = chunk_coord * CHUNK_SIZE as i32;
        let chunk_corner_distance = chunk_corner.as_::<f32>() - self.position;
        let view_plane_normal = self.look_at();

        let corners = [
            Vec3::zero(),
            Vec3::new(0, 0, 1),
            Vec3::new(0, 1, 0),
            Vec3::new(0, 1, 1),
            Vec3::new(1, 0, 0),
            Vec3::new(1, 1, 0),
            Vec3::new(1, 0, 1),
            Vec3::new(1, 1, 1),
        ];

        corners.into_iter().any(|c| {
            view_plane_normal.dot(chunk_corner_distance + c.as_::<f32>() * CHUNK_SIZE as f32) >= 0.0
        })
    }
}
