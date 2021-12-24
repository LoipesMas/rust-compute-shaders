use glow::*;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;

#[derive(Default, Clone, Copy, Debug)]
struct CellData {
    alive: u32,
    lifetime: f32,
    creation: f32,
}

pub struct StructuredBuffer<T>
where
    T: Default + Clone,
{
    phantom: std::marker::PhantomData<T>,

    id: Buffer,
    buffer_size: usize,
    elements: usize,
    data: Vec<T>,
}

pub fn main() {
    unsafe {
        let (gl, window, event_loop) = {
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
            let gl =
                glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);
            (gl, window, event_loop)
        };

        let field_size = (16, 16);

        let mut image_data = vec![];
        image_data.resize_with(field_size.0 * field_size.1, || {
            let mut c = CellData::default();
            c.alive = 1;
            c.lifetime = 10000.0;
            c.creation = 0.0;
            c
        });

        let buffer_size = std::mem::size_of::<CellData>() * field_size.0 * field_size.1;

        let image_data_u8 : &[u8] = core::slice::from_raw_parts(
            image_data.as_ptr() as *const u8,
            image_data.len() * core::mem::size_of::<CellData>(),
        );

        let id1 = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(id1));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            image_data_u8,
            glow::DYNAMIC_COPY,
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

        //let prev_sb = StructuredBuffer {
        //    phantom: std::marker::PhantomData,
        //    id,
        //    buffer_size,
        //    elements: image_data.len(),
        //    data: image_data,
        //};

        let mut image_data_2 = vec![];
        image_data_2.resize(field_size.0 * field_size.1, CellData::default());

        let buffer_size = std::mem::size_of::<CellData>() * field_size.0 * field_size.1;

        let image_data_u8_2: &[u8] = core::slice::from_raw_parts(
            image_data_2.as_ptr() as *const u8,
            image_data_2.len() * core::mem::size_of::<CellData>(),
        );

        let id2 = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(id2));
        gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            image_data_u8_2,
            glow::DYNAMIC_COPY,
        );
        gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

        //let curr_sb = StructuredBuffer {
        //    phantom: std::marker::PhantomData,
        //    id,
        //    buffer_size,
        //    elements: image_data.len(),
        //    data: image_data,
        //};

        let program = gl.create_program().expect("Cannot create program");
        let shader_source = include_str!("shader.vsh");
        let shader = gl.create_shader(glow::COMPUTE_SHADER).unwrap();
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }
        gl.attach_shader(program, shader);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }
        let mut t = 0.0;
        let mut flip = 1;
        gl.use_program(Some(program));
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            t += 0.01666;
            match event {
                Event::LoopDestroyed => {
                    return;
                }
                Event::MainEventsCleared => {
                    window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    gl.use_program(Some(program));
                    let loc1 = gl.get_uniform_location(program, "u_field_size");
                    let loc2 = gl.get_uniform_location(program, "u_dt");
                    let loc3 = gl.get_uniform_location(program, "u_time");

                    gl.uniform_2_f32(loc1.as_ref(), field_size.0 as f32, field_size.1 as f32);
                    gl.uniform_1_f32(loc2.as_ref(), 0.01666);
                    gl.uniform_1_f32(loc3.as_ref(), t);

                    gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, flip, Some(id1));
                    gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1 - flip, Some(id2));
                    flip = 1 - flip;

                    gl.dispatch_compute(field_size.0 as u32 / 8, field_size.1 as u32 / 8, 1);
                    gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, None);
                    gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, None);
                    gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
                    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(id2));
                    {
                        let p = gl.map_buffer_range(
                            glow::SHADER_STORAGE_BUFFER,
                            0,
                            std::mem::size_of::<CellData>().try_into().unwrap(),
                            glow::MAP_READ_BIT,
                        ) as *mut CellData;
                        let slice =
                            { std::slice::from_raw_parts(p, 1) };
                        println!("e {:?}", gl.get_error());
                        println!("p {:?}", p);
                        println!("s {:?}", slice);
                        gl.unmap_buffer(glow::SHADER_STORAGE_BUFFER);
                    }
                    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);
                    //println!("a {:?}", image_data[1]);
                    //println!("b {:?}", image_data_2[1]);
                    //println!("c {:?}", &image_data_u8[0..10]);
                    window.swap_buffers().unwrap();
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        window.resize(*physical_size);
                    }
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => (),
                },
                _ => (),
            }
        });
    }
}

/*
impl Computor {
    pub fn new() -> Self {
        unsafe {
            let (gl, window) = {
                let window = glutin::ContextBuilder::new()
                    .with_vsync(true)
                    .build_osmesa(glutin::dpi::PhysicalSize {
                        width: 256,
                        height: 256,
                    })
                    .unwrap()
                    .make_current()
                    .unwrap();
                let gl =
                    glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);
                (gl, window)
            };

            let field_size = (10, 10);

            let mut image_data = vec![];
            image_data.resize(field_size.0 * field_size.1, CellData::default());

            let buffer_size = std::mem::size_of::<CellData>() * field_size.0 * field_size.1;

            let image_data_u8: &[u8] = core::slice::from_raw_parts(
                image_data.as_ptr() as *const u8,
                image_data.len() * core::mem::size_of::<CellData>(),
            );

            let id = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(id));
            gl.buffer_data_u8_slice(
                glow::SHADER_STORAGE_BUFFER,
                image_data_u8,
                glow::DYNAMIC_COPY,
            );
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

            let prev_sb = StructuredBuffer {
                phantom: std::marker::PhantomData,
                id,
                buffer_size,
                elements: image_data.len(),
                data: image_data,
            };

            let mut image_data = vec![];
            image_data.resize(field_size.0 * field_size.1, CellData::default());

            let buffer_size = std::mem::size_of::<CellData>() * field_size.0 * field_size.1;

            let image_data_u8: &[u8] = core::slice::from_raw_parts(
                image_data.as_ptr() as *const u8,
                image_data.len() * core::mem::size_of::<CellData>(),
            );

            let id = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(id));
            gl.buffer_data_u8_slice(
                glow::SHADER_STORAGE_BUFFER,
                image_data_u8,
                glow::DYNAMIC_COPY,
            );
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

            let curr_sb = StructuredBuffer {
                phantom: std::marker::PhantomData,
                id,
                buffer_size,
                elements: image_data.len(),
                data: image_data,
            };

            let program = gl.create_program().expect("Cannot create program");

            //let shader_source = r#"#version 440
            //    layout(local_size_x = 1 ,local_size_y = 1) in;
            //    layout(rgba8, binding = 0) uniform image2D img_output;
            //    uniform sampler2D u_texture;
            //    void main() {
            //        ivec2 pixel_coord = ivec2(gl_GlobalInvocationID.xy);
            //        float curr = texelFetch(u_texture,pixel_coord,0).r;
            //        if (curr > 1.0) {
            //            curr -= 1.0;
            //        }
            //        curr += 0.016;
            //        vec4 pixel        = vec4(curr,0.0,0.0,1.0);
            //        imageStore(img_output, pixel_coord, pixel);
            //    }
            //"#;
            let shader_source = include_str!("shader.vsh");
            let shader = gl.create_shader(glow::COMPUTE_SHADER).unwrap();
            gl.shader_source(shader, shader_source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!("{}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }
            gl.use_program(Some(program));
            Self {
                gl,
                window,
                field_size,
                program,
                prev_sb,
                curr_sb,
            }
        }
    }

    pub fn process(&mut self, _delta: f64) {
        unsafe {
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.use_program(Some(self.program));

            self.gl
                .bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.curr_sb.id));
            self.gl
                .bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.prev_sb.id));

            for _ in 0..=10240 {
                self.gl.dispatch_compute(
                    self.field_size.0 as u32 / 8,
                    self.field_size.1 as u32 / 8,
                    1,
                );
            }
            self.gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
            println!("{}", self.gl.get_error());
            println!("d {:?}", self.curr_sb.data[0]);
            println!("d2 {:?}", self.prev_sb.data[0]);
        }
    }

    pub unsafe fn gl_thread(
        gl: Context,
        window: glutin::Context<glutin::PossiblyCurrent>,
        event_loop: EventLoop<()>,
    ) {
        println!("TEST1");
        {
            use glutin::event::{Event, WindowEvent};
            use glutin::event_loop::ControlFlow;
            println!("TEST");
            event_loop.run(move |event, _, control_flow| {
                println!("TEST2");
                *control_flow = ControlFlow::Wait;
                *control_flow = ControlFlow::Exit;
                match event {
                    Event::LoopDestroyed => {
                        return;
                    }
                    Event::MainEventsCleared => {
                        //window.window().request_redraw();
                    }
                    Event::RedrawRequested(_) => {
                        gl.clear(glow::COLOR_BUFFER_BIT);
                        //gl.draw_arrays(glow::TRIANGLES, 0, 3);
                    }
                    Event::WindowEvent { ref event, .. } => {
                        if event == &WindowEvent::CloseRequested {
                            //gl.delete_program(program);
                            //gl.delete_vertex_array(vertex_array);
                            *control_flow = ControlFlow::Exit
                        }
                    }
                    _ => (),
                }
            });
        }
    }
}
*/
