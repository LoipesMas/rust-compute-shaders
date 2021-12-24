use glow::*;

#[derive(Default, Clone, Copy, Debug)]
struct CellData {
    val: f32,
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

        let field_size = (1024, 1024);

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
        for _ in 0..120 {
            //std::thread::sleep_ms(17);
            gl.use_program(Some(program));
            let loc1 = gl.get_uniform_location(program, "u_field_size");

            let t1 = std::time::Instant::now();
            gl.uniform_2_f32(loc1.as_ref(), field_size.0 as f32, field_size.1 as f32);

            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(out_data));
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(in_data));

            gl.dispatch_compute(field_size.0 as u32 / 8, field_size.1 as u32 / 8, 1);
            std::mem::swap(&mut out_data, &mut in_data);
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);
            println!("t {:?}", t1.elapsed());
        }
            gl.memory_barrier(glow::SHADER_STORAGE_BARRIER_BIT);
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(out_data));
            {
                let p = gl.map_buffer_range(
                    glow::SHADER_STORAGE_BUFFER,
                    0,
                    std::mem::size_of::<CellData>() as i32 * 3,
                    glow::MAP_READ_BIT,
                ) as *mut CellData;
                let slice = { std::slice::from_raw_parts(p, 3) };
                println!("s {:?}", slice);
                gl.unmap_buffer(glow::SHADER_STORAGE_BUFFER);
            }
    }
}
