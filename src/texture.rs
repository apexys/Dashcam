use gl::types::*;
use gl;
use std::os::raw::c_void;


pub struct Texture{
    pub handle: GLuint,
    pub width: usize,
    pub height: usize,
    pub alpha: bool
}

impl Texture{
    pub fn new(width: usize, height: usize, alpha: bool) -> Texture{
        let mut textureId = 0;
        let data = vec![0u8; width * height * if alpha {4} else {3}];
        let format = match alpha {
            false => gl::RGB,
            true => gl::RGBA
        };
        

        unsafe{
            gl::GenTextures(1, &mut textureId);
            gl::BindTexture(gl::TEXTURE_2D, textureId);
            gl::TexImage2D(gl::TEXTURE_2D, 0, format as i32, width as i32, height as i32,
                0, format, gl::UNSIGNED_BYTE, &data[0] as *const u8 as *const c_void);
            gl::GenerateMipmap(gl::TEXTURE_2D);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        }
        Texture{
            handle: textureId,
            width,
            height,
            alpha
        }
    }

    pub fn update(&self, data: &[u8]){
        if data.is_empty(){
            return;
        }
        let format = match self.alpha {
            false => gl::RGB,
            true => gl::RGBA
        };
        unsafe{
            gl::BindTexture(gl::TEXTURE_2D, self.handle);
            gl::TexImage2D(gl::TEXTURE_2D, 0, format as i32, self.width as i32, self.height as i32,
                0, format, gl::UNSIGNED_BYTE, &data[0] as *const u8 as *const c_void);
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
    }
}
