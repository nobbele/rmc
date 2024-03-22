use glow::HasContext;

unsafe fn compile_shader(gl: &glow::Context, shader_type: u32, source: &str) -> glow::Shader {
    let shader = gl.create_shader(shader_type).expect("Cannot create shader");
    gl.shader_source(shader, source);
    gl.compile_shader(shader);
    if !gl.get_shader_compile_status(shader) {
        panic!("{}", gl.get_shader_info_log(shader));
    }
    shader
}

pub unsafe fn create_shader(gl: &glow::Context, vertex: &str, fragment: &str) -> glow::Program {
    let program = gl.create_program().expect("Cannot create program");

    let vertex = compile_shader(gl, glow::VERTEX_SHADER, vertex);
    let fragment = compile_shader(gl, glow::FRAGMENT_SHADER, fragment);
    gl.attach_shader(program, vertex);
    gl.attach_shader(program, fragment);

    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("{}", gl.get_program_info_log(program));
    }

    gl.detach_shader(program, vertex);
    gl.detach_shader(program, fragment);
    gl.delete_shader(vertex);
    gl.delete_shader(fragment);

    program
}
