use glow::*;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;

use rand::prelude::*;

#[derive(Default, Clone, Copy, Debug)]
struct CellData {
    position: [f32; 2],
    velocity: [f32; 2],
    group: [i32; 2],
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

        let count = 1024;

        let speed = 0.6;

        let mut image_data = vec![];
        image_data.resize_with(count, || {
            let mut c = CellData::default();
            c.position[0] = random::<f32>();
            c.position[1] = random::<f32>();
            let angle = (random::<f32>() - 0.5) * 2.0 * std::f32::consts::PI;
            c.velocity[0] = angle.cos() * speed;
            c.velocity[1] = angle.sin() * speed;
            c.group[0] = (random::<f32>() * 3.0) as i32;
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

        let mut out_data = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(out_data));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            image_data_u8,
            glow::DYNAMIC_COPY,
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

        let compute_program = gl.create_program().expect("Cannot create program");
        let shader_source = include_str!("shader.comp");
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
                gl_Position = vec4((vert ), 0.0, 1.0);
            }"#,
            r#"#version 440
            struct CellData{
                vec2 position;
                vec2 velocity;
                ivec2 group;
            };


            uniform int u_count;

            layout(shared, binding = 0) readonly buffer Data
            {
                CellData data[];
            };

            precision mediump float;
            
            in vec2 vert;
            out vec4 color;
            void main() {
                vec2 v = vert;
                for (int i = 0; i < u_count; i++){
                    CellData boid = data[i];
                    vec2 dir = v-boid.position;
                    if (length(dir) < 0.005){
                        dir = normalize(dir);
                        color = vec4(dot(dir,normalize(boid.velocity)), float(boid.group.x) / 3.0, 0.5,1.0);
                        return;
                    }
                }
                color = vec4(1.0,1.0,1.0,1.0);
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
        let count_loc = gl.get_uniform_location(compute_program, "u_count");
        let dt_loc = gl.get_uniform_location(compute_program, "u_dt");
        let time_loc = gl.get_uniform_location(compute_program, "u_time");
        let count_tex_loc = gl.get_uniform_location(tex_program, "u_count");

        let mut t1 = std::time::Instant::now();
        gl.clear_color(0.95, 0.75, 0.75, 1.0);
        let mut t = 0.0;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::MainEventsCleared => {
                    window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let dt = t1.elapsed().as_secs_f32();
                    t += dt;
                    t1 = std::time::Instant::now();
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    // DRAWING AND COMPUTING
                    {
                        gl.viewport(-640, -940, 1920, 1920);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
                        gl.use_program(Some(compute_program));
                        gl.uniform_1_i32(count_loc.as_ref(), count as i32);
                        gl.uniform_1_f32(dt_loc.as_ref(), dt);
                        gl.uniform_1_f32(time_loc.as_ref(), t);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(out_data));
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(in_data));

                        gl.dispatch_compute(count as u32 / 32, 1, 1);
                        std::mem::swap(&mut out_data, &mut in_data);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);

                        gl.use_program(Some(tex_program));
                        gl.uniform_1_i32(count_tex_loc.as_ref(), count as i32);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(in_data));
                        gl.bind_vertex_array(Some(vertex_array));
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    }
                    // for debugging
                    if false {
                        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(in_data));
                        {
                            let p = gl.map_buffer_range(
                                glow::SHADER_STORAGE_BUFFER,
                                0,
                                (std::mem::size_of::<CellData>() * count) as i32,
                                glow::MAP_READ_BIT,
                            ) as *mut CellData;
                            let slice = { std::slice::from_raw_parts(p, count) };
                            println!("s {:?}", &slice[..3]);
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
                        gl.delete_program(tex_program);
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
