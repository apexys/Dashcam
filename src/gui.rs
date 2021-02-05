use arc_swap::ArcSwap;
use gl::types::*;
use glutin::{Api, GlRequest, dpi};
use std::{ffi::CString, sync::Arc};
use std::mem;
use std::ptr;
use std::str;
use crate::{Program, Shader, texture::Texture};

const IMAGE_WIDTH: usize = 1280;
const IMAGE_HEIGHT: usize = 720;
const IMAGE_BYTES_PER_PIXEL: usize = 3;



const VERTEX_DATA: [GLfloat; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];

const VERTEX_SHADER_SOURCE: &'static str = "
attribute vec2 position;
attribute vec2 vertexUV;
//uniform vec2 ul_corner;
//uniform vec2 size;

varying vec2 UV;

void main(){
    gl_Position = vec4(position, 0.0, 1.0);
    UV = (position + 1.0) / 2.0;
}
";

const FRAGMENT_SHADER_SOURCE: &'static str = "
precision highp float;
varying vec2 UV;
uniform sampler2D texture1;

void main(){
    gl_FragColor = texture2D(texture1, UV);
}
";

pub struct Gui{

}

impl Gui{
    pub fn start(camera_image: Arc<ArcSwap<Vec<u8>>>){
        let event_loop = glutin::event_loop::EventLoop::new();
        let window = glutin::window::WindowBuilder::new().with_inner_size(dpi::LogicalSize::new(1280, 720));
        let gl_window = glutin::ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGlEs, (2,0)))
            .build_windowed(window, &event_loop)
            .unwrap();
    
        // It is essential to make the context current before calling `gl::load_with`.
        let gl_window = unsafe { gl_window.make_current() }.unwrap();
    
        // Load the OpenGL function pointers
        gl::load_with(|symbol| gl_window.get_proc_address(symbol));
    
        // Create GLSL shaders
        let vertex = Shader::new(VERTEX_SHADER_SOURCE, gl::VERTEX_SHADER);
        let fragment = Shader::new(FRAGMENT_SHADER_SOURCE,gl::FRAGMENT_SHADER);
        let program = Program::new(vertex, fragment);
    
        //Create vao and vbo
        let mut vao = 0;
        let mut vbo = 0;
        unsafe{
            // Create Vertex Array Object
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );
        }


        //Create texture
        let tex = Texture::new(1280, 720, false);

        //Use shader program
        unsafe{
            // Use shader program
            gl::UseProgram(program.handle);
            //gl::BindFragDataLocation(program.handle, 0, CString::new("color").unwrap().as_ptr());
        }

        //Enable vertex position attribute array
        unsafe{
            let pos_attr = gl::GetAttribLocation(program.handle, CString::new("position").unwrap().as_ptr());
            gl::EnableVertexAttribArray(pos_attr as GLuint);
            gl::VertexAttribPointer(
                pos_attr as GLuint,
                2,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                0,
                ptr::null(),
            );
        }

        event_loop.run(move |event, _, control_flow| {
            use glutin::event::{Event, WindowEvent};
            use glutin::event_loop::ControlFlow;
            *control_flow = ControlFlow::Poll;
            match event {
                Event::LoopDestroyed => return,
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        // Cleanup
                        unsafe {
                            gl::DeleteBuffers(1, &vbo);
                            gl::DeleteVertexArrays(1, &vao);
                        }
                        *control_flow = ControlFlow::Exit
                    },
                    WindowEvent::Resized(size) => {
                        unsafe{
                            gl::Viewport(0,0, size.width as i32, size.height as i32);
                        }
                    }
                    _ => (),
                },
                Event::RedrawRequested(_) => {
                    tex.update(&camera_image.load());
                    unsafe {
                        // Clear the screen to black
                        gl::ClearColor(0.3, 0.3, 0.3, 1.0);
                        gl::Clear(gl::COLOR_BUFFER_BIT);
                        // Draw a triangle from the 3 vertices
                        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
                    }
                    gl_window.swap_buffers().unwrap();
                },
                _ => {
                    gl_window.window().request_redraw();
                },
            }
        });
    }

}