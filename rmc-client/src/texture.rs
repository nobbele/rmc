use glow::HasContext;

pub enum DataSource<'a, T: ?Sized> {
    Path(&'a str),
    Inline(&'a T),
}

pub unsafe fn load_texture(gl: &glow::Context, data_source: DataSource<'_, [u8]>) -> glow::Texture {
    let image = match data_source {
        DataSource::Inline(bytes) => image::load_from_memory(bytes).unwrap(),
        DataSource::Path(_) => panic!(),
    }
    .to_rgba8();
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA8 as _,
        image.width() as _,
        image.height() as _,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        Some(image.into_iter().as_slice()),
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
    texture
}

pub unsafe fn load_array_texture(
    gl: &glow::Context,
    data_sources: &[DataSource<'_, [u8]>],
) -> glow::Texture {
    let images = data_sources
        .iter()
        .map(|data_source| {
            match data_source {
                DataSource::Inline(bytes) => image::load_from_memory(bytes).unwrap(),
                DataSource::Path(_) => panic!(),
            }
            .to_rgb8()
        })
        .collect::<Vec<_>>();

    assert_ne!(images.len(), 0);
    for image in &images {
        assert_eq!(image.width(), images[0].width());
        assert_eq!(image.height(), images[0].height());
    }

    let block_array_texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(block_array_texture));
    gl.tex_storage_3d(
        glow::TEXTURE_2D_ARRAY,
        1,
        glow::RGB8,
        images[0].width() as _,
        images[0].height() as _,
        images.len() as _,
    );
    for (i, image) in images.into_iter().enumerate() {
        gl.tex_sub_image_3d(
            glow::TEXTURE_2D_ARRAY,
            0,
            0,
            0,
            i as _,
            image.width() as _,
            image.height() as _,
            1,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(image.into_iter().as_slice()),
        );
    }
    gl.generate_mipmap(glow::TEXTURE_2D_ARRAY);
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_MAG_FILTER,
        glow::NEAREST as _,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_MIN_FILTER,
        glow::NEAREST as _,
    );
    block_array_texture
}
