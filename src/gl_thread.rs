use glow::*;
use glutin::event_loop::EventLoop;
use glutin::platform::unix::HeadlessContextExt;

pub struct Computor {
    gl: Context,
    window: glutin::Context<glutin::PossiblyCurrent>,
    program: Program,
    field_size: (usize, usize),
    prev_sb: StructuredBuffer<CellData>,
    curr_sb: StructuredBuffer<CellData>,
}

#[derive(Default, Clone, Copy, Debug)]
struct CellData{
    alive : bool,
    lifetime : f32,
    creation : f32,
}

pub struct StructuredBuffer<T>
        where T: Default + Clone {
    phantom: std::marker::PhantomData<T>,

    id : Buffer,
    buffer_size : usize,
    elements: usize,
    data: Vec<T>,
}

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
            gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, image_data_u8, glow::DYNAMIC_COPY);
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
            gl.buffer_data_u8_slice(glow::SHADER_STORAGE_BUFFER, image_data_u8, glow::DYNAMIC_COPY);
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


            self.gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.curr_sb.id));
            self.gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.prev_sb.id));

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
