use vek::Vec2;

pub mod chunk_renderer;
pub use chunk_renderer::ChunkRenderer;

pub mod screen_quad_renderer;
pub use screen_quad_renderer::ScreenQuadRenderer;

pub mod isometric_block_renderer;
pub use isometric_block_renderer::IsometricBlockRenderer;

pub mod game_renderer;
pub use game_renderer::GameRenderer;

pub mod text_renderer;
pub use text_renderer::TextRenderer;

fn face_to_tri(v: &[u8; 4]) -> [u8; 6] {
    [v[0], v[1], v[3], v[3], v[2], v[0]]
}

// pub enum ScaleOrSize {
//     Scale(Vec2<f32>),
//     Size(Vec2<f32>),
// }

#[derive(Clone, Copy)]
pub struct DrawParams {
    pub position: Vec2<f32>,
    pub origin: Vec2<f32>,
    pub scale: Vec2<f32>,
}

impl DrawParams {
    pub fn scale(mut self, scale: Vec2<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn position(mut self, position: Vec2<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn origin(mut self, origin: Vec2<f32>) -> Self {
        self.origin = origin;
        self
    }
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            position: Vec2::zero(),
            origin: Vec2::zero(),
            scale: Vec2::one(),
        }
    }
}
