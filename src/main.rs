mod wificam;
use wificam::WifiCam;

fn to_hex_str(bytes: &[u8]) -> String{
    bytes.iter().map(|b| format!("{:02X}",b)).collect()
}

fn main() {
    let cam = WifiCam::new();
    cam.run();
}
