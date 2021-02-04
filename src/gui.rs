use gl::types::*;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::str;
use crate::{Program, Shader};

const IMAGE_WIDTH: usize = 1280;
const IMAGE_HEIGHT: usize = 720;
const IMAGE_BYTES_PER_PIXEL: usize = 3;



const VERTEX_DATA: [Glfloat; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];

const VERTEX_SHADER_SOURCE: &'static str = "
#version 430
layout(location = 0) in vec2 position;
layout(location = 1) in vec2 vertexUV;
uniform vec2 ul_corner;
uniform vec2 size;

out vec2 UV;

void main(){
    gl_Position = vec4(position, 0.0, 1.0);
    UV = position;
}
";

const FRAGMENT_SHADER_SOURCE: &'static str = "
in vec2 UV;
uniform sampler2D texture1;

out vec4 color;

void main(){
    color = texture(texture1, UV);
}
";

pub struct Gui{

}

impl Gui{
    pub fn start(){
        let event_loop = glutin::event_loop::EventLoop::new();
        let window = glutin::window::WindowBuilder::new();
        let gl_window = glutin::ContextBuilder::new()
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
        let mut textureId = 0;
        let data = vec![0u8; IMAGE_WIDTH * IMAGE_HEIGHT * IMAGE_BYTES_PER_PIXEL];
        let format = gl::RGB;
        unsafe{
            gl::GenTextures(1, &mut textureId);
            gl::BindTexture(gl::TEXTURE_2D, textureId);
            gl::TexImage2D(gl::TEXTURE_2D, 0, format as i32, img.width() as i32, img.height() as i32,
                0, format, gl::UNSIGNED_BYTE, &data[0] as *const u8 as *const c_void);
            gl::GenerateMipmap(gl::TEXTURE_2D);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        }

        //Use shader program
        unsafe{
            // Use shader program
            gl::UseProgram(program.handle);
            gl::BindFragDataLocation(program.handle, 0, CString::new("color").unwrap().as_ptr());
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

    }


    pub fn 
}