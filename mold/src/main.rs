use glow::*;
use imgui_winit_support::WinitPlatform;

use glutin::{event::Event, event_loop::ControlFlow, WindowedContext};

use rand::random;

type Window = WindowedContext<glutin::PossiblyCurrent>;

#[derive(Default, Clone, Copy, Debug)]
struct CellData {
    position: [f32; 2],
    angle: [f32; 2],
}

#[derive(Default, Clone, Copy, Debug)]
struct TexCellData {
    #[allow(dead_code)]
    color: f32,
}

pub fn main() {
    unsafe {
        let event_loop = glutin::event_loop::EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title("Mold")
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

        let mut tex_size = 4096u32;
        let mut count = 2u32.pow(17);

        let mut in_data = gl.create_buffer().unwrap();
        let mut out_data = gl.create_buffer().unwrap();
        let mut tex_data = gl.create_buffer().unwrap();
        let mut tex_data_2 = gl.create_buffer().unwrap();

        new_simulation(gl, count, tex_size, in_data, out_data, tex_data, tex_data_2);

        let compute_program = gl.create_program().expect("Cannot create program");

        let shader_source = &std::fs::read_to_string("mold/src/shader.comp").unwrap()[..];
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

        let blur_program = gl.create_program().expect("Cannot create program");
        let shader_source = &std::fs::read_to_string("mold/src/blur.comp").unwrap()[..];
        let shader = gl.create_shader(glow::COMPUTE_SHADER).unwrap();
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }
        gl.attach_shader(blur_program, shader);
        gl.link_program(blur_program);
        if !gl.get_program_link_status(blur_program) {
            panic!("{}", gl.get_program_info_log(blur_program));
        }
        gl.detach_shader(blur_program, shader);
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
            struct TexCellData{
                float color;
            };


            uniform ivec2 u_field_size;

            layout(shared, binding = 0) readonly buffer Data
            {
                TexCellData data[];
            };

            int GetArrayId(ivec2 pos)
            {
                return pos.x + pos.y * int(u_field_size.x);
            }

            precision mediump float;

            uniform vec3 color_1 = vec3(0.1, 0.1, 0.2);
            uniform vec3 color_2 = vec3(1.0, 0.6, 0);
            uniform vec3 color_3 = vec3(1.0, 0.6, 0);
            uniform float color_step = 0.5;
            
            in vec2 vert;
            out vec4 color;
            void main() {
	            ivec2 pixel_cord = ivec2(vert * u_field_size);
                TexCellData tc = data[GetArrayId(pixel_cord)];
                if (tc.color > color_step)
                    color = vec4(mix(color_2, color_3, (tc.color-color_step)/(1.0-color_step)),tc.color);
                else
                    color = vec4(mix(color_1, color_2, tc.color/color_step),tc.color);
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
        let dt_loc = gl.get_uniform_location(compute_program, "u_dt").unwrap();
        let speed_loc = gl.get_uniform_location(compute_program, "u_speed").unwrap();
        let turn_speed_loc = gl
            .get_uniform_location(compute_program, "u_turn_speed")
            .unwrap();
        let sensor_size_loc = gl
            .get_uniform_location(compute_program, "u_sensor_size")
            .unwrap();
        let sensor_stride_loc = gl
            .get_uniform_location(compute_program, "u_sensor_stride")
            .unwrap();
        let sensor_offset_loc = gl
            .get_uniform_location(compute_program, "u_sensor_offset")
            .unwrap();
        let trail_weight_loc = gl
            .get_uniform_location(compute_program, "u_trail_weight")
            .unwrap();
        let field_size_loc = gl
            .get_uniform_location(compute_program, "u_field_size")
            .unwrap();
        let field_size_tex_loc = gl
            .get_uniform_location(tex_program, "u_field_size")
            .unwrap();
        let color_1_tex_loc = gl.get_uniform_location(tex_program, "color_1").unwrap();
        let color_2_tex_loc = gl.get_uniform_location(tex_program, "color_2").unwrap();
        let color_3_tex_loc = gl.get_uniform_location(tex_program, "color_3").unwrap();
        let color_step_tex_loc = gl.get_uniform_location(tex_program, "color_step").unwrap();
        let field_size_blur_loc = gl
            .get_uniform_location(blur_program, "u_field_size")
            .unwrap();
        let dt_blur_loc = gl.get_uniform_location(blur_program, "u_dt").unwrap();
        let decay_rate_blur_loc = gl
            .get_uniform_location(blur_program, "u_decay_rate")
            .unwrap();

        let mut speed = 200.0;
        let mut turn_speed = 40.0;
        let mut sensor_size = 2;
        let mut sensor_stride = 2;
        let mut sensor_offset = 30;
        let mut trail_weight = 10;
        let mut decay_rate = 0.2;
        let mut blur_passes = 1;

        let mut color_1 = [0.1, 0.1, 0.2];
        let mut color_2 = [0.7, 0.3, 0.15];
        let mut color_3 = [1.0, 0.6, 0.0];
        let mut color_step = 0.5;

        let mut new_tex_size = tex_size;
        let mut new_count = count;

        let mut t1 = std::time::Instant::now();
        let mut dt = t1.elapsed().as_secs_f32();
        gl.clear_color(0.95, 0.75, 0.75, 1.0);
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(_) => {
                    imgui_context.io_mut().update_delta_time(t1.elapsed());
                    dt = t1.elapsed().as_secs_f32();
                    t1 = std::time::Instant::now();
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
                        // Computing
                        gl.viewport(-640, -940, 1920, 1920);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
                        gl.use_program(Some(compute_program));
                        gl.uniform_1_f32(Some(&dt_loc), dt);
                        gl.uniform_1_f32(Some(&speed_loc), speed);
                        gl.uniform_1_f32(Some(&turn_speed_loc), turn_speed);
                        gl.uniform_1_i32(Some(&sensor_size_loc), sensor_size);
                        gl.uniform_1_i32(Some(&sensor_stride_loc), sensor_stride);
                        gl.uniform_1_i32(Some(&sensor_offset_loc), sensor_offset);
                        gl.uniform_1_i32(Some(&trail_weight_loc), trail_weight);
                        gl.uniform_2_i32(Some(&field_size_loc), tex_size as i32, tex_size as i32);
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(out_data));
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(in_data));
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 2, Some(tex_data));

                        gl.dispatch_compute(count as u32 / 32, 1, 1);
                        std::mem::swap(&mut out_data, &mut in_data);
                        gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);

                        //Bluring
                        gl.use_program(Some(blur_program));
                        gl.uniform_1_f32(Some(&dt_blur_loc), dt);
                        gl.uniform_1_f32(Some(&decay_rate_blur_loc), decay_rate);
                        gl.uniform_2_i32(
                            Some(&field_size_blur_loc),
                            tex_size as i32,
                            tex_size as i32,
                        );
                        for _ in 0..blur_passes {
                            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(tex_data_2));
                            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 2, Some(tex_data));
                            gl.dispatch_compute(tex_size as u32 / 8, tex_size as u32 / 8, 1);
                            std::mem::swap(&mut tex_data, &mut tex_data_2);
                            gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
                            gl.uniform_1_f32(Some(&decay_rate_blur_loc), 0.0);
                        }

                        // Drawing
                        gl.use_program(Some(tex_program));
                        gl.uniform_3_f32_slice(Some(&color_1_tex_loc), &color_1);
                        gl.uniform_3_f32_slice(Some(&color_2_tex_loc), &color_2);
                        gl.uniform_3_f32_slice(Some(&color_3_tex_loc), &color_3);
                        gl.uniform_1_f32(Some(&color_step_tex_loc), color_step);
                        gl.uniform_2_i32(
                            Some(&field_size_tex_loc),
                            tex_size as i32,
                            tex_size as i32,
                        );
                        gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(tex_data));
                        gl.bind_vertex_array(Some(vertex_array));
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    }

                    let mut reset_sim = false;
                    let mut reset_settings = false;
                    let ui = imgui_context.frame();
                    imgui::Window::new("Simulation settings")
                        .size([480.0, 380.0], imgui::Condition::Always)
                        .position([1300.0, 100.0], imgui::Condition::Always)
                        .resizable(false)
                        .movable(false)
                        .build(&ui, || {
                            imgui::Slider::new("Speed", 0.0, 600.0).build(&ui, &mut speed);

                            imgui::Slider::new("Turn speed", 0.0, 100.0)
                                .build(&ui, &mut turn_speed);

                            imgui::Slider::new("Sensor size", 0, 4).build(&ui, &mut sensor_size);

                            imgui::Slider::new("Sensor stride", 0, 4)
                                .build(&ui, &mut sensor_stride);

                            imgui::Slider::new("Sensor offset", 0, 50)
                                .build(&ui, &mut sensor_offset);

                            imgui::Slider::new("Trail weight", 0, 50).build(&ui, &mut trail_weight);

                            imgui::Slider::new("Decay rate", 0.0, 3.0).build(&ui, &mut decay_rate);

                            imgui::Slider::new("Blur passes", 0, 5).build(&ui, &mut blur_passes);

                            ui.separator();
                            ui.text("These settings require simulation reset:");
                            imgui::Slider::new("Agent count", 32, 262144)
                                .build(&ui, &mut new_count);
                            new_count -= new_count % 32; // Snap to multiple of 32
                            imgui::Slider::new("Texture size", 8, 4096)
                                .build(&ui, &mut new_tex_size);
                            new_tex_size -= new_tex_size % 8; // Snap to multiple of 8
                            ui.separator();

                            reset_sim = ui.button("Reset simulation");
                            ui.same_line();
                            reset_settings = ui.button("Reset settings");

                            imgui::ColorEdit::new("Background color", &mut color_1).build(&ui);
                            imgui::ColorEdit::new("Color 1", &mut color_2).build(&ui);
                            imgui::ColorEdit::new("Color 2", &mut color_3).build(&ui);
                            imgui::Slider::new("Color step", 0.001, 1.0)
                                .build(&ui, &mut color_step);
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
                        tex_size = new_tex_size;
                        count = new_count;
                        new_simulation(
                            ig_renderer.gl_context(),
                            count,
                            tex_size,
                            in_data,
                            out_data,
                            tex_data,
                            tex_data_2,
                        );
                    }
                    // Reset settings
                    if reset_settings {
                        speed = 200.0;
                        turn_speed = 40.0;
                        sensor_size = 2;
                        sensor_stride = 2;
                        sensor_offset = 30;
                        trail_weight = 10;
                        decay_rate = 0.2;
                        blur_passes = 1;
                        tex_size = 4096;
                        new_tex_size = tex_size;
                        count = 2u32.pow(17);
                        new_count = count;
                    }
                }
                glutin::event::Event::WindowEvent {
                    event: glutin::event::WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }
                event => {
                    winit_platform.handle_event(imgui_context.io_mut(), window.window(), &event);
                }
            }
        });
    }
}

unsafe fn new_simulation(
    gl: &Context,
    count: u32,
    tex_size: u32,
    in_data: Buffer,
    out_data: Buffer,
    tex_data: Buffer,
    tex_data_2: Buffer,
) {
    let mut agents_data = vec![];
    agents_data.resize_with(count as usize, || {
        let mut c = CellData::default();
        let angle = (random::<f32>() - 0.5) * 2.0 * std::f32::consts::PI;
        c.angle[0] = -angle;
        let dist = random::<f32>().sqrt() * 0.3;
        c.position = [angle.cos() * dist + 0.5, angle.sin() * dist + 0.5];
        c
    });

    let agents_data_u8: &[u8] = core::slice::from_raw_parts(
        agents_data.as_ptr() as *const u8,
        agents_data.len() * core::mem::size_of::<CellData>(),
    );

    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(in_data));
    gl.buffer_data_u8_slice(
        glow::SHADER_STORAGE_BUFFER,
        agents_data_u8,
        glow::DYNAMIC_COPY,
    );
    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(out_data));
    gl.buffer_data_u8_slice(
        glow::SHADER_STORAGE_BUFFER,
        agents_data_u8,
        glow::DYNAMIC_COPY,
    );

    let tex_data_data = {
        let mut v = vec![];
        v.resize((tex_size * tex_size) as usize, TexCellData::default());
        v
    };
    let text_data_data_u8: &[u8] = core::slice::from_raw_parts(
        tex_data_data.as_ptr() as *const u8,
        tex_data_data.len() * core::mem::size_of::<TexCellData>(),
    );
    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(tex_data));
    gl.buffer_data_u8_slice(
        glow::SHADER_STORAGE_BUFFER,
        text_data_data_u8,
        glow::DYNAMIC_COPY,
    );
    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(tex_data_2));
    gl.buffer_data_u8_slice(
        glow::SHADER_STORAGE_BUFFER,
        text_data_data_u8,
        glow::DYNAMIC_COPY,
    );
    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);
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
