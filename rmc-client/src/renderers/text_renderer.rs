use std::mem;

use ab_glyph::FontVec;
use glow::HasContext;
use glyph_brush::{BrushAction, BrushError, Extra, GlyphBrush, GlyphBrushBuilder, GlyphCruncher};
use vek::{Aabr, Mat3, Vec2, Vec4};

use crate::shader::create_shader;

use super::DrawParams;

#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct TextVertex {
    pub left_top: Vec2<f32>,
    pub right_bottom: Vec2<f32>,
    pub tex_left_top: Vec2<f32>,
    pub tex_right_bottom: Vec2<f32>,
    pub color: Vec4<f32>,
}

unsafe impl bytemuck::Pod for TextVertex {}
unsafe impl bytemuck::Zeroable for TextVertex {}

pub struct TextRenderer {
    pub vao: glow::VertexArray,
    pub ib: glow::Buffer,

    pub texture: glow::Texture,
    pub program: glow::Program,

    pub glyph_brush: GlyphBrush<TextVertex, Extra, FontVec>,
    pub glyph_count: usize,

    pub section: glyph_brush::Section<'static>,
}

impl TextRenderer {
    pub unsafe fn new(gl: &glow::Context, section: glyph_brush::Section<'static>) -> Self {
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(
            0,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<TextVertex>() as _,
            mem::offset_of!(TextVertex, left_top) as _,
        );
        gl.vertex_attrib_divisor(0, 1);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(
            1,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<TextVertex>() as _,
            mem::offset_of!(TextVertex, right_bottom) as _,
        );
        gl.vertex_attrib_divisor(1, 1);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_f32(
            2,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<TextVertex>() as _,
            mem::offset_of!(TextVertex, tex_left_top) as _,
        );
        gl.vertex_attrib_divisor(2, 1);
        gl.enable_vertex_attrib_array(3);
        gl.vertex_attrib_pointer_f32(
            3,
            2,
            glow::FLOAT,
            false,
            mem::size_of::<TextVertex>() as _,
            mem::offset_of!(TextVertex, tex_right_bottom) as _,
        );
        gl.vertex_attrib_divisor(3, 1);
        gl.enable_vertex_attrib_array(4);
        gl.vertex_attrib_pointer_f32(
            4,
            4,
            glow::FLOAT,
            false,
            mem::size_of::<TextVertex>() as _,
            mem::offset_of!(TextVertex, color) as _,
        );
        gl.vertex_attrib_divisor(4, 1);

        let program = create_shader(
            &gl,
            include_str!("../../shaders/text.vert"),
            include_str!("../../shaders/text.frag"),
        );

        let font =
            FontVec::try_from_vec(Vec::from(include_bytes!("../../fonts/Cute Dino.otf"))).unwrap();
        let mut glyph_brush = GlyphBrushBuilder::using_font(font).build();
        glyph_brush.queue(&section);

        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8 as _,
            glyph_brush.texture_dimensions().0 as _,
            glyph_brush.texture_dimensions().1 as _,
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            None,
        );

        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as _,
        );

        let mut r = TextRenderer {
            vao,
            ib: vbo,
            texture,
            program,
            glyph_brush,
            glyph_count: 0,
            section,
        };

        r.flush(gl);

        r
    }

    pub fn set_section(&mut self, section: glyph_brush::Section<'static>) {
        self.section = section;
        self.glyph_brush.queue(&self.section)
    }

    pub unsafe fn flush(&mut self, gl: &glow::Context) {
        let update_texture = |rect: glyph_brush::Rectangle<u32>, tex_data: &[u8]| {
            let offset = Vec2::new(rect.min[0], rect.min[1]);
            let size = Vec2::new(rect.width(), rect.height());

            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                offset.x as _,
                offset.y as _,
                size.x as _,
                size.y as _,
                glow::RED,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(tex_data),
            );
        };

        match self.glyph_brush.process_queued(update_texture, to_vertex) {
            Ok(BrushAction::Draw(vertices)) => {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.ib));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(vertices.as_slice()),
                    glow::STATIC_DRAW,
                );

                self.glyph_count = vertices.len();
            }
            Ok(BrushAction::ReDraw) => {}
            Err(BrushError::TextureTooSmall { suggested }) => {
                panic!("resize {:?}", suggested);
            }
        }
    }

    pub unsafe fn draw(&mut self, gl: &glow::Context, params: DrawParams) {
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        let text_size = self
            .glyph_brush
            .glyph_bounds(&self.section)
            .map(|rect| Vec2::new(rect.width(), rect.height()))
            .unwrap_or_default();

        let screen_to_view_scale = Vec2::one() / Vec2::new(1024.0, 768.0);
        let mvp = Mat3::<f32>::identity()
            * Mat3::translation_2d(params.position * screen_to_view_scale)
            * Mat3::scaling_3d((screen_to_view_scale * params.scale).with_z(1.0))
            * Mat3::translation_2d(-params.origin * text_size);

        gl.use_program(Some(self.program));
        gl.uniform_matrix_3_f32_slice(
            Some(
                &gl.get_uniform_location(self.program, "uniform_Transform")
                    .unwrap(),
            ),
            false,
            mvp.as_col_slice(),
        );

        gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, self.glyph_count as _);

        gl.disable(glow::BLEND);
    }
}

fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        extra,
    }: glyph_brush::GlyphVertex,
) -> TextVertex {
    let mut gl_rect = Aabr {
        min: Vec2::new(pixel_coords.min.x, pixel_coords.min.y),
        max: Vec2::new(pixel_coords.max.x, pixel_coords.max.y),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > bounds.max.x {
        let old_width = gl_rect.size().w;
        gl_rect.max.x = bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.size().w / old_width;
    }
    if gl_rect.min.x < bounds.min.x {
        let old_width = gl_rect.size().w;
        gl_rect.min.x = bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.size().w / old_width;
    }
    if gl_rect.max.y > bounds.max.y {
        let old_height = gl_rect.size().h;
        gl_rect.max.y = bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.size().h / old_height;
    }
    if gl_rect.min.y < bounds.min.y {
        let old_height = gl_rect.size().h;
        gl_rect.min.y = bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.size().h / old_height;
    }

    TextVertex {
        left_top: Vec2::new(gl_rect.min.x, gl_rect.max.y),
        right_bottom: Vec2::new(gl_rect.max.x, gl_rect.min.y),
        tex_left_top: Vec2::new(tex_coords.min.x, tex_coords.max.y),
        tex_right_bottom: Vec2::new(tex_coords.max.x, tex_coords.min.y),
        color: extra.color.into(),
    }
}
