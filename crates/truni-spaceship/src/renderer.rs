use crate::scene::SpaceshipScene;
use glam::Mat4;
use glow::{Context, HasContext};
use std::sync::Arc;

const VERTEX_SHADER_SRC: &str = r#"#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;

uniform mat4 uModel;
uniform mat4 uView;
uniform mat4 uProjection;

out vec3 FragPos;
out vec3 Normal;

void main() {
    FragPos = vec3(uModel * vec4(aPos, 1.0));
    Normal = mat3(transpose(inverse(uModel))) * aNormal;
    gl_Position = uProjection * uView * vec4(FragPos, 1.0);
}
"#;

const FRAGMENT_SHADER_SRC: &str = r#"#version 330 core
out vec4 FragColor;

in vec3 FragPos;
in vec3 Normal;

uniform vec4 uBaseColor;
uniform vec4 uEmissiveColor;
uniform float uEmissiveIntensity;
uniform int uIsSelected;

void main() {
    vec3 norm = normalize(Normal);
    vec3 lightDir1 = normalize(vec3(1.5, 2.5, 2.0));
    vec3 lightDir2 = normalize(vec3(-2.0, -1.0, -1.5));

    // Directional lighting
    float diff1 = max(dot(norm, lightDir1), 0.0);
    float diff2 = max(dot(norm, lightDir2), 0.0) * 0.3;
    float diffuse = diff1 + diff2;

    vec3 ambient = uBaseColor.rgb * 0.25;
    vec3 lighting = uBaseColor.rgb * diffuse + ambient;

    // Emissive glowing technical veins
    vec3 emissive = uEmissiveColor.rgb * uEmissiveIntensity;

    vec3 finalColor = lighting + emissive;

    // Highlight selected components with bright holographic edge
    if (uIsSelected == 1) {
        finalColor += vec3(0.0, 0.9, 1.0) * 0.6;
    }

    FragColor = vec4(finalColor, uBaseColor.a);
}
"#;

pub struct SpaceshipRenderer {
    gl: Arc<Context>,
    program: glow::Program,
    cube_vao: glow::VertexArray,
    _cube_vbo: glow::Buffer,
}

impl SpaceshipRenderer {
    pub unsafe fn new(gl: Arc<Context>) -> Result<Self, String> {
        let program = create_shader_program(&gl, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC)?;

        // Setup unit cube geometry with normals
        #[rustfmt::skip]
        let vertices: [f32; 216] = [
            // Back face
            -0.5, -0.5, -0.5,  0.0,  0.0, -1.0,
             0.5, -0.5, -0.5,  0.0,  0.0, -1.0,
             0.5,  0.5, -0.5,  0.0,  0.0, -1.0,
             0.5,  0.5, -0.5,  0.0,  0.0, -1.0,
            -0.5,  0.5, -0.5,  0.0,  0.0, -1.0,
            -0.5, -0.5, -0.5,  0.0,  0.0, -1.0,
            // Front face
            -0.5, -0.5,  0.5,  0.0,  0.0,  1.0,
             0.5, -0.5,  0.5,  0.0,  0.0,  1.0,
             0.5,  0.5,  0.5,  0.0,  0.0,  1.0,
             0.5,  0.5,  0.5,  0.0,  0.0,  1.0,
            -0.5,  0.5,  0.5,  0.0,  0.0,  1.0,
            -0.5, -0.5,  0.5,  0.0,  0.0,  1.0,
            // Left face
            -0.5,  0.5,  0.5, -1.0,  0.0,  0.0,
            -0.5,  0.5, -0.5, -1.0,  0.0,  0.0,
            -0.5, -0.5, -0.5, -1.0,  0.0,  0.0,
            -0.5, -0.5, -0.5, -1.0,  0.0,  0.0,
            -0.5, -0.5,  0.5, -1.0,  0.0,  0.0,
            -0.5,  0.5,  0.5, -1.0,  0.0,  0.0,
            // Right face
             0.5,  0.5,  0.5,  1.0,  0.0,  0.0,
             0.5,  0.5, -0.5,  1.0,  0.0,  0.0,
             0.5, -0.5, -0.5,  1.0,  0.0,  0.0,
             0.5, -0.5, -0.5,  1.0,  0.0,  0.0,
             0.5, -0.5,  0.5,  1.0,  0.0,  0.0,
             0.5,  0.5,  0.5,  1.0,  0.0,  0.0,
            // Bottom face
            -0.5, -0.5, -0.5,  0.0, -1.0,  0.0,
             0.5, -0.5, -0.5,  0.0, -1.0,  0.0,
             0.5, -0.5,  0.5,  0.0, -1.0,  0.0,
             0.5, -0.5,  0.5,  0.0, -1.0,  0.0,
            -0.5, -0.5,  0.5,  0.0, -1.0,  0.0,
            -0.5, -0.5, -0.5,  0.0, -1.0,  0.0,
            // Top face
            -0.5,  0.5, -0.5,  0.0,  1.0,  0.0,
             0.5,  0.5, -0.5,  0.0,  1.0,  0.0,
             0.5,  0.5,  0.5,  0.0,  1.0,  0.0,
             0.5,  0.5,  0.5,  0.0,  1.0,  0.0,
            -0.5,  0.5,  0.5,  0.0,  1.0,  0.0,
            -0.5,  0.5, -0.5,  0.0,  1.0,  0.0
        ];

        let cube_vao = gl.create_vertex_array().map_err(|e| e.to_string())?;
        let cube_vbo = gl.create_buffer().map_err(|e| e.to_string())?;

        gl.bind_vertex_array(Some(cube_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(cube_vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck_cast_slice(&vertices),
            glow::STATIC_DRAW,
        );

        let stride = 6 * std::mem::size_of::<f32>() as i32;
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(0);

        gl.vertex_attrib_pointer_f32(
            1,
            3,
            glow::FLOAT,
            false,
            stride,
            (3 * std::mem::size_of::<f32>()) as i32,
        );
        gl.enable_vertex_attrib_array(1);

        gl.bind_vertex_array(None);

        Ok(Self {
            gl,
            program,
            cube_vao,
            _cube_vbo: cube_vbo,
        })
    }

    pub unsafe fn render(&self, scene: &SpaceshipScene, width: i32, height: i32) {
        let gl = &self.gl;

        gl.viewport(0, 0, width, height);
        gl.enable(glow::DEPTH_TEST);
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        // Dark deep space sci-fi background
        gl.clear_color(0.015, 0.03, 0.05, 0.98);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

        gl.use_program(Some(self.program));

        let aspect = (width as f32) / (height.max(1) as f32);
        let view = scene.camera.view_matrix();
        let proj = scene.camera.projection_matrix(aspect);

        let u_view = gl.get_uniform_location(self.program, "uView");
        let u_proj = gl.get_uniform_location(self.program, "uProjection");
        let u_model = gl.get_uniform_location(self.program, "uModel");
        let u_base = gl.get_uniform_location(self.program, "uBaseColor");
        let u_emissive = gl.get_uniform_location(self.program, "uEmissiveColor");
        let u_intensity = gl.get_uniform_location(self.program, "uEmissiveIntensity");
        let u_selected = gl.get_uniform_location(self.program, "uIsSelected");

        gl.uniform_matrix_4_f32_slice(u_view.as_ref(), false, &view.to_cols_array());
        gl.uniform_matrix_4_f32_slice(u_proj.as_ref(), false, &proj.to_cols_array());

        gl.bind_vertex_array(Some(self.cube_vao));

        // Render each component of the spacecraft
        for comp in &scene.components {
            let model = Mat4::from_translation(comp.position) * Mat4::from_scale(comp.size);
            gl.uniform_matrix_4_f32_slice(u_model.as_ref(), false, &model.to_cols_array());

            gl.uniform_4_f32(
                u_base.as_ref(),
                comp.base_color[0],
                comp.base_color[1],
                comp.base_color[2],
                comp.base_color[3],
            );
            gl.uniform_4_f32(
                u_emissive.as_ref(),
                comp.emissive_color[0],
                comp.emissive_color[1],
                comp.emissive_color[2],
                comp.emissive_color[3],
            );
            gl.uniform_1_f32(u_intensity.as_ref(), comp.emissive_intensity);
            gl.uniform_1_i32(u_selected.as_ref(), if comp.is_selected { 1 } else { 0 });

            gl.draw_arrays(glow::TRIANGLES, 0, 36);
        }

        gl.bind_vertex_array(None);
        gl.use_program(None);
    }
}

fn bytemuck_cast_slice(data: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<f32>(),
        )
    }
}

unsafe fn create_shader_program(
    gl: &Context,
    vert_src: &str,
    frag_src: &str,
) -> Result<glow::Program, String> {
    let program = gl.create_program().map_err(|e| e.to_string())?;

    let vs = compile_shader(gl, glow::VERTEX_SHADER, vert_src)?;
    let fs = compile_shader(gl, glow::FRAGMENT_SHADER, frag_src)?;

    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);

    if !gl.get_program_link_status(program) {
        return Err(gl.get_program_info_log(program));
    }

    gl.delete_shader(vs);
    gl.delete_shader(fs);

    Ok(program)
}

unsafe fn compile_shader(gl: &Context, shader_type: u32, src: &str) -> Result<glow::Shader, String> {
    let shader = gl.create_shader(shader_type).map_err(|e| e.to_string())?;
    gl.shader_source(shader, src);
    gl.compile_shader(shader);

    if !gl.get_shader_compile_status(shader) {
        return Err(gl.get_shader_info_log(shader));
    }

    Ok(shader)
}
