use glow::*;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;

#[derive(Default, Clone, Copy, Debug)]
struct CellData {
    val: f32,
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

        let field_size = (16, 16);

        let mut image_data = vec![];
        let mut x = 0.0;
        image_data.resize_with(field_size.0 * field_size.1, || {
            let mut c = CellData::default();
            x += 1.0;
            c.val = x;
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

        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(in_data));
        {
            let p = gl.map_buffer_range(
                glow::SHADER_STORAGE_BUFFER,
                0,
                std::mem::size_of::<CellData>() as i32 * 3,
                glow::MAP_READ_BIT,
            ) as *mut CellData;
            let slice = { std::slice::from_raw_parts(p, 3) };
            println!("s1 {:?}", slice);
            gl.unmap_buffer(glow::SHADER_STORAGE_BUFFER);
        }

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
        let shader_source = include_str!("shader.vsh");
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
        println!("S: {}", slice.len());
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
        //gl.tex_image_2d(
        //    glow::TEXTURE_2D,
        //    0,
        //    glow::RGBA8 as i32,
        //    field_size.0 as i32,
        //    field_size.1 as i32,
        //    0,
        //    glow::RGBA8,
        //    glow::UNSIGNED_BYTE,
        //    Some(slice),
        //);
        println!("ee2 {:?}", gl.get_error());
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
                gl_Position = vec4(vert - 0.5, 0.0, 1.0);
            }"#,
            r#"#version 440
            struct CellData{
                float val;
            };


            uniform vec2 u_field_size;

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

                ivec2 pixel_coord = ivec2(floor(temp));

                CellData data = data[GetArrayId(pixel_coord)];
                color = vec4(data.val, 1.0, 1.0, 1.0);
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

        //std::thread::sleep_ms(17);
        gl.use_program(Some(compute_program));
        let loc1 = gl.get_uniform_location(compute_program, "u_field_size");
        let loc2 = gl.get_uniform_location(tex_program, "u_field_size");

        let t1 = std::time::Instant::now();

        println!("t {:?}", t1.elapsed());

        gl.clear_color(0.95, 0.75, 0.75, 1.0);
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::MainEventsCleared => {
                    window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    // DRAWING
                    {
                        gl.use_program(Some(compute_program));
                        gl.uniform_2_f32(loc1.as_ref(), field_size.0 as f32, field_size.1 as f32);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(out_data));
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(in_data));
                        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                        gl.bind_image_texture(2, tex, 1, false, 0, glow::WRITE_ONLY, glow::RGBA32F);
                        println!("E: {:?}", gl.get_error());

                        gl.dispatch_compute(field_size.0 as u32 / 8, field_size.1 as u32 / 8, 1);
                        std::mem::swap(&mut out_data, &mut in_data);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);

                        gl.use_program(Some(tex_program));
                        gl.uniform_2_f32(loc2.as_ref(), field_size.0 as f32, field_size.1 as f32);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(in_data));
                        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                        gl.bind_vertex_array(Some(vertex_array));
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

                        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(out_data));
                        {
                            let p = gl.map_buffer_range(
                                glow::SHADER_STORAGE_BUFFER,
                                0,
                                (std::mem::size_of::<CellData>() * field_size.0 * field_size.1) as i32,
                                glow::MAP_READ_BIT,
                            ) as *mut CellData;
                            let slice = { std::slice::from_raw_parts(p, field_size.0 * field_size.1) };
                            println!("s {:?}", slice[9]);
                            gl.unmap_buffer(glow::SHADER_STORAGE_BUFFER);
                        }
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
                    _ => (),
                },
                _ => (),
            }
        });
    }
}
