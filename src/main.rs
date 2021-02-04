mod wificam;
use wificam::WifiCam;
mod program;
use program::{Shader, Program};
mod gui;
use gui::Gui;
mod texture;
use texture::Texture;

fn to_hex_str(bytes: &[u8]) -> String{
    bytes.iter().map(|b| format!("{:02X}",b)).collect()
}

fn main() {
    let cam = WifiCam::new();
    cam.run();
}
