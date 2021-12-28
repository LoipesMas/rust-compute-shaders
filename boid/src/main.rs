use glow::*;
use imgui_winit_support::WinitPlatform;

use glutin::{event::Event, event_loop::ControlFlow, WindowedContext};

use rand::random;

type Window = WindowedContext<glutin::PossiblyCurrent>;
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
            .with_title("Boids")
            .with_fullscreen(Some(glutin::window::Fullscreen::Borderless(None)))
            .with_inner_size(glutin::dpi::LogicalSize::new(1920.0, 1080.0));
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)
            .unwrap()
            .make_current()
            .unwrap();
        let gl = glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);

        let (mut winit_platform, mut imgui_context) = imgui_init(&window);

        let mut ig_renderer = imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui_context)
            .expect("failed to create renderer");

        let gl = ig_renderer.gl_context();

        let mut count = 1024u32;

        let mut speed = 0.4;

        let mut in_data = gl.create_buffer().unwrap();
        let mut out_data = gl.create_buffer().unwrap();
        new_simulation(gl, count, in_data, out_data);

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
            uniform float u_size;

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
                    if (length(dir) < u_size*0.5){
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
        let speed_loc = gl.get_uniform_location(compute_program, "u_speed");
        let vision_loc = gl.get_uniform_location(compute_program, "u_vision");
        let size_loc = gl.get_uniform_location(compute_program, "u_close_range");
        let dt_loc = gl.get_uniform_location(compute_program, "u_dt");
        let count_tex_loc = gl.get_uniform_location(tex_program, "u_count");
        let size_tex_loc = gl.get_uniform_location(tex_program, "u_size");

        let mut t1 = std::time::Instant::now();
        gl.clear_color(0.95, 0.75, 0.75, 1.0);
        let mut dt = t1.elapsed().as_secs_f32();
        let mut new_count = count;
        let mut size = 0.01;
        let mut vision = 0.09;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(_) => {
                    let dtd = t1.elapsed();
                    dt = dtd.as_secs_f32();
                    t1 = std::time::Instant::now();
                    imgui_context.io_mut().update_delta_time(dtd);
                }
                Event::MainEventsCleared => {
                    winit_platform
                        .prepare_frame(imgui_context.io_mut(), window.window())
                        .unwrap();
                    window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let gl = ig_renderer.gl_context();
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    // DRAWING AND COMPUTING
                    {
                        gl.viewport(-640, -940, 1920, 1920);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
                        gl.use_program(Some(compute_program));
                        gl.uniform_1_i32(count_loc.as_ref(), count as i32);
                        gl.uniform_1_f32(dt_loc.as_ref(), dt);
                        gl.uniform_1_f32(speed_loc.as_ref(), speed);
                        gl.uniform_1_f32(vision_loc.as_ref(), vision);
                        gl.uniform_1_f32(size_loc.as_ref(), size);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(out_data));
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(in_data));

                        gl.dispatch_compute(count as u32 / 8, 1, 1);
                        std::mem::swap(&mut out_data, &mut in_data);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);

                        gl.use_program(Some(tex_program));
                        gl.uniform_1_i32(count_tex_loc.as_ref(), count as i32);
                        gl.uniform_1_f32(size_tex_loc.as_ref(), size);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(in_data));
                        gl.bind_vertex_array(Some(vertex_array));
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    }

                    let mut reset_sim = false;
                    let mut reset_settings = false;
                    let ui = imgui_context.frame();
                    imgui::Window::new("Simulation settings")
                        .size([480.0, 300.0], imgui::Condition::Always)
                        .position([1300.0, 100.0], imgui::Condition::Always)
                        .resizable(false)
                        .movable(false)
                        .build(&ui, || {
                            imgui::Slider::new("Speed", 0.01, 1.0)
                                .range(0.01, 1.0)
                                .build(&ui, &mut speed);
                            imgui::Slider::new("Vision range", 0.00, 0.2)
                                .range(0.00, 0.2)
                                .build(&ui, &mut vision);
                            imgui::Slider::new("Size", 0.001, 0.015)
                                .range(0.001, 0.015)
                                .build(&ui, &mut size);

                            ui.separator();
                            ui.text("These settings require simulation reset:");
                            imgui::Slider::new("Agent count", 8, 1024)
                                .range(8, 1024)
                                .build(&ui, &mut new_count);
                            new_count -= new_count % 8; // Snap to multiple of 32
                            ui.separator();

                            reset_sim = ui.button("Reset simulation");
                            ui.same_line();
                            reset_settings = ui.button("Reset settings");
                        });

                    winit_platform.prepare_render(&ui, window.window());
                    let draw_data = ui.render();

                    // This is the only extra render step to add
                    ig_renderer
                        .render(draw_data)
                        .expect("error rendering imgui");
                    window.swap_buffers().unwrap();

                    // Reset simulation
                    if reset_sim {
                        count = new_count;
                        new_simulation(
                            ig_renderer.gl_context(),
                            count,
                            in_data,
                            out_data,
                        );
                    }
                    // Reset settings
                    if reset_settings {
                        speed = 0.6;
                        count = 1024;
                        new_count = count;
                    }
                }
                glutin::event::Event::WindowEvent {
                    event: glutin::event::WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                },
                event => {
                    winit_platform.handle_event(imgui_context.io_mut(), window.window(), &event);
                }
            }
        });
    }
}


unsafe fn new_simulation(gl: &Context, count: u32, in_data: Buffer, out_data: Buffer) {
        let mut agent_data = vec![];
        agent_data.resize_with(count as usize, || {
            let mut c = CellData::default();
            c.position[0] = random::<f32>();
            c.position[1] = random::<f32>();
            let angle = (random::<f32>() - 0.5) * 2.0 * std::f32::consts::PI;
            c.velocity[0] = angle.cos();
            c.velocity[1] = angle.sin();
            c.group[0] = (random::<f32>() * 3.0) as i32;
            c
        });

        let agent_data_u8: &[u8] = core::slice::from_raw_parts(
            agent_data.as_ptr() as *const u8,
            agent_data.len() * core::mem::size_of::<CellData>(),
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(in_data));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            agent_data_u8,
            glow::DYNAMIC_COPY,
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(out_data));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            agent_data_u8,
            glow::DYNAMIC_COPY,
        );


}


fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(
        imgui_context.io_mut(),
        window.window(),
        imgui_winit_support::HiDpiMode::Rounded,
    );

    imgui_context
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    (winit_platform, imgui_context)
}
