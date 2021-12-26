use glow::*;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;

use rand::prelude::*;

#[derive(Default, Clone, Copy, Debug)]
struct CellData {
    alive: u32,
    lifetime: f32,
}

pub fn main() {
    unsafe {
        let event_loop = glutin::event_loop::EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title("Hello wo!")
            .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)
            .unwrap()
            .make_current()
            .unwrap();
        let gl = glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);

        let s = 1024;
        let field_size = (s, s);

        let mut image_data = vec![];
        image_data.resize_with(field_size.0 * field_size.1, || {
            let mut c = CellData::default();
            if random::<f32>() < 0.1 {
                c.alive = 1;
                c.lifetime = 1.0;
            }
            c
        });

        let image_data_u8: &[u8] = core::slice::from_raw_parts(
            image_data.as_ptr() as *const u8,
            image_data.len() * core::mem::size_of::<CellData>(),
        );

        let mut in_data = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(in_data));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            image_data_u8,
            glow::DYNAMIC_COPY,
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

        let mut image_data_2 = vec![];
        image_data_2.resize(field_size.0 * field_size.1, CellData::default());

        let image_data_u8_2: &[u8] = core::slice::from_raw_parts(
            image_data_2.as_ptr() as *const u8,
            image_data_2.len() * core::mem::size_of::<CellData>(),
        );

        let mut out_data = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(out_data));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            image_data_u8_2,
            glow::DYNAMIC_COPY,
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

        let compute_program = gl.create_program().expect("Cannot create program");
        let shader_source = &std::fs::read_to_string("src/shader.comp").unwrap()[..];
        let shader = gl.create_shader(glow::COMPUTE_SHADER).unwrap();
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }
        gl.attach_shader(compute_program, shader);
        gl.link_program(compute_program);
        if !gl.get_program_link_status(compute_program) {
            panic!("{}", gl.get_program_info_log(compute_program));
        }
        gl.detach_shader(compute_program, shader);
        gl.delete_shader(shader);

        // FOR TEXTURE DRAWING
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        let data_ = vec![255u8; field_size.0 * field_size.1 * 4];
        let slice = { std::slice::from_raw_parts::<u8>(data_.as_ptr(), data_.len()) };
        gl.tex_storage_2d(
            glow::TEXTURE_2D,
            1,
            glow::RGBA32F,
            field_size.0 as i32,
            field_size.1 as i32,
        );
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            field_size.0 as i32,
            field_size.1 as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(slice),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);

        let vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(vertex_array));

        let tex_program = gl.create_program().expect("Cannot create program");

        let (vertex_shader_source, fragment_shader_source) = (
            r#"#version 440
            const vec2 verts[4] = vec2[4](
                vec2(0.0f, 1.0f),
                vec2(1.0f, 1.0f),
                vec2(0.0f, 0.0f),
                vec2(1.0f, 0.0f)
            );
            out vec2 vert;
            void main() {
                vert = verts[gl_VertexID];
                gl_Position = vec4((vert - 0.5), 0.0, 1.0);
            }"#,
            r#"#version 440
            struct CellData{
                bool alive;
                float lifetime;
            };


            uniform vec2 u_field_size;
            uniform float u_zoom;

            int GetArrayId(ivec2 pos)
            {
                return pos.x + pos.y * int(u_field_size.x);
            }

            layout(shared, binding = 0) readonly buffer Data
            {
                CellData data[];
            };

            precision mediump float;
            
            in vec2 vert;
            out vec4 color;
            void main() {
                vec2 temp = vert;
                temp.x *= u_field_size.x;
                temp.y *= u_field_size.y;
                ivec2 pixel_coord = ivec2(floor(temp / u_zoom));

                CellData data = data[GetArrayId(pixel_coord)];
                float lt = max(data.lifetime, 0.0);
                lt *= lt;
                if (lt > 0.5) {
                    color = vec4(mix(vec3(0.8,0.6,lt), vec3(0.3,0.8,0.6), (lt-0.5)*2.0), 1.0);
                } else {
                    color = vec4(vec3(0.2,0.1,0.7), 1.0);
                }
            }"#,
        );

        let shader_sources = [
            (glow::VERTEX_SHADER, vertex_shader_source),
            (glow::FRAGMENT_SHADER, fragment_shader_source),
        ];

        let mut shaders = Vec::with_capacity(shader_sources.len());

        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(shader, shader_source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!("{}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(tex_program, shader);
            shaders.push(shader);
        }

        gl.link_program(tex_program);
        if !gl.get_program_link_status(tex_program) {
            panic!("{}", gl.get_program_info_log(tex_program));
        }

        for shader in shaders {
            gl.detach_shader(tex_program, shader);
            gl.delete_shader(shader);
        }
        // END TEXTURE DRAWING

        gl.use_program(Some(compute_program));
        let field_size_loc = gl.get_uniform_location(compute_program, "u_field_size");
        let dt_loc = gl.get_uniform_location(compute_program, "u_dt");
        let field_size_tex_loc = gl.get_uniform_location(tex_program, "u_field_size");
        let zoom_loc = gl.get_uniform_location(tex_program, "u_zoom");

        let mut t1 = std::time::Instant::now();
        gl.clear_color(0.95, 0.75, 0.75, 1.0);
        let mut zoom = 1.0;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::MainEventsCleared => {
                    window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let dt = t1.elapsed().as_secs_f32();
                    t1 = std::time::Instant::now();
                    gl.viewport(-240, -440, 1920, 1920);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    // DRAWING AND COMPUTING
                    {
                        gl.use_program(Some(compute_program));
                        gl.uniform_2_f32(
                            field_size_loc.as_ref(),
                            field_size.0 as f32,
                            field_size.1 as f32,
                        );
                        gl.uniform_1_f32(dt_loc.as_ref(), dt);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(out_data));
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(in_data));
                        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                        gl.bind_image_texture(2, tex, 1, false, 0, glow::WRITE_ONLY, glow::RGBA32F);

                        gl.dispatch_compute(field_size.0 as u32 / 32, field_size.1 as u32 / 32, 1);
                        std::mem::swap(&mut out_data, &mut in_data);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);

                        gl.use_program(Some(tex_program));
                        gl.uniform_2_f32(
                            field_size_tex_loc.as_ref(),
                            field_size.0 as f32,
                            field_size.1 as f32,
                        );
                        gl.uniform_1_f32(zoom_loc.as_ref(), zoom);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(in_data));
                        gl.bind_vertex_array(Some(vertex_array));
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    }
                    window.swap_buffers().unwrap();
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        window.resize(*physical_size);
                    }
                    WindowEvent::CloseRequested => {
                        gl.delete_program(compute_program);
                        gl.delete_buffer(in_data);
                        gl.delete_buffer(out_data);
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(keycode) = input.virtual_keycode {
                            if keycode == glutin::event::VirtualKeyCode::I {
                                zoom = 20.0f32.min(zoom * 1.1);
                            } else if keycode == glutin::event::VirtualKeyCode::O {
                                zoom = 1.0f32.max(zoom * 0.9);
                            }
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        });
    }
}
