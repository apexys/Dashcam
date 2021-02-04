use gl::types::*;
use std::ffi::CString;
use std::ptr;
use std::str;


pub struct Shader{
    pub handle: GLuint,
    pub source: String
}

impl Shader{
    pub fn new(source: &str, shader_type: GLenum) -> Shader{
        let handle = Shader::compile_shader(source, shader_type);
        let source = String::from(source);
        Shader{
            handle,
            source
        }
    }

    fn compile_shader(src: &str, ty: GLenum) -> GLuint {
        let shader;
        unsafe {
            shader = gl::CreateShader(ty);
            // Attempt to compile the shader
            let c_str = CString::new(src.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);
    
            // Get the compile status
            let mut status = gl::FALSE as GLint;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    
            // Fail on error
            if status != (gl::TRUE as GLint) {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = Vec::with_capacity(len as usize);
                buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    ptr::null_mut(),
                    buf.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "{}",
                    str::from_utf8(&buf)
                        .ok()
                        .expect("ShaderInfoLog not valid utf8")
                );
            }
        }
        shader
    }
}

impl Drop for Shader{
    fn drop(&mut self) {
        unsafe{
            gl::DeleteShader(self.handle)
        };
    }
}

pub struct Program{
    pub handle: GLuint,
    _vertex_shader: Shader,
    _fragment_shader: Shader
}

impl Program{
    pub fn new(vertex_shader: Shader, fragment_shader: Shader)-> Program{
        let handle = Program::link_program(vertex_shader.handle,fragment_shader.handle);
        Program{
            handle,
            _vertex_shader: vertex_shader,
            _fragment_shader: fragment_shader
        }
    }

    fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
        unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::LinkProgram(program);
            // Get the link status
            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    
            // Fail on error
            if status != (gl::TRUE as GLint) {
                let mut len: GLint = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = Vec::with_capacity(len as usize);
                buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
                gl::GetProgramInfoLog(
                    program,
                    len,
                    ptr::null_mut(),
                    buf.as_mut_ptr() as *mut GLchar,
                );
                panic!(
                    "{}",
                    str::from_utf8(&buf)
                        .ok()
                        .expect("ProgramInfoLog not valid utf8")
                );
            }
            program
        }
    }
}

impl Drop for Program{
    fn drop(&mut self) {
        unsafe{
            gl::DeleteProgram(self.handle);
        }
    }
}