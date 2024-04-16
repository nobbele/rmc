pub mod chunk_renderer;
pub use chunk_renderer::ChunkRenderer;

pub mod screen_quad_renderer;
pub use screen_quad_renderer::ScreenQuadRenderer;

pub mod isometric_block_renderer;
pub use isometric_block_renderer::IsometricBlockRenderer;

pub mod game_renderer;
pub use game_renderer::GameRenderer;

fn face_to_tri(v: &[u8; 4]) -> [u8; 6] {
    [v[0], v[1], v[3], v[3], v[2], v[0]]
}
