use std::{collections::VecDeque, net::{TcpStream, UdpSocket}, thread::{self, JoinHandle}, time::{Duration, SystemTime, UNIX_EPOCH}};
use std::sync::mpsc::{channel, Sender,Receiver};
use jpeg_decoder::Decoder;
use ringbuf::{RingBuffer, Producer, Consumer};
use smallvec::SmallVec;
use std::io::prelude::*;

pub struct WifiCam{
    udp_thread: JoinHandle<()>,
    tcp_thread: JoinHandle<()>,
    jpeg_thread: JoinHandle<()>,
    tcp_messages: Receiver<TcpMessage>,
}

impl WifiCam{
    pub fn run(self){
        self.udp_thread.join();
        self.tcp_thread.join();
        self.jpeg_thread.join();
    }

    pub fn new() -> WifiCam{
        let (udp_thread, consumer) = WifiCam::start_udp_receiver();
        let (tcp_thread, tcp_messages) = WifiCam::send_init_sequence();
        let jpeg_thread = WifiCam::start_jpeg_thread(consumer);
        WifiCam{
            udp_thread,
            tcp_thread,
            tcp_messages,
            jpeg_thread,
        }
    }

    fn start_udp_receiver() -> (JoinHandle<()>, Consumer<u8>){
        //Create ringbuffer to communicate with decoder
        let ringbuffer = RingBuffer::new(1024 * 1024);
        let (mut producer, mut consumer) = ringbuffer.split();
        //Spawn udp receiver thread
        let udp_thread = thread::spawn(move || {
            //Bind to address
            let socket = UdpSocket::bind("192.168.1.2:5555").expect("Error binding to UDP socket");
            //Receive packets and write to ringbuffer
            let mut buf = vec![0; 1024 * 32];
            loop{
                let (length, src) = socket.recv_from(&mut buf).expect("Error receiving UDP data");
                producer.write_all(&buf[0..length]).expect("Error writing to ringbuffer");
                //eprintln!("Received {} bytes from {}", length, src);
            }
        });
        
        (udp_thread, consumer)
    }

    fn send_init_sequence() -> (JoinHandle<()>,Receiver<TcpMessage>){
        //Create channel to keep a hold of tcp messages
        let (sender, receiver) = channel::<TcpMessage>();
        //Spawn tcp transceiver thread
        let tcp_thread = thread::spawn(move || {
            //Magic sequence captured from the app MRT_Camera with wireshark
            const init_sequence: [u8; 20] = [
                0x01, 0x01, 0x02, 0x10,
                0x02, 0x01, 0x03, 0x20, 0x02, 0x01, 0x03, 0x20,
                0x0e, 0x01, 0xaf, 0xe0, 0x24, 0x01, 0xc0, 0x42
            ];
            //Connect to tcp port
            let mut stream = TcpStream::connect("192.168.1.1:5252").expect("Error binding to TCP socket");
            //Write magic sequence
            let length_written = stream.write(&init_sequence).expect("Error sending TCP data");
            eprintln!("Wrote {}/{} bytes to remote", length_written, init_sequence.len());
            //Start reading
            loop {
                let mut buf = [0;256];
                let length_read = stream.read(&mut buf).expect("Error reading TCP data");
                //After receiving, transform message into TcpMessage and pass on
                let message: TcpMessage = buf[0..length_read].into();
                eprintln!("Received {} bytes from remote TCP (Message = {:?})",length_read, message);
                sender.send(message).expect("Error sending TCP message to queue");
            }
        });
        
        (tcp_thread, receiver)
    }

    fn start_jpeg_thread(mut bytestream: Consumer<u8>) -> JoinHandle<()>{
        let jpeg_thread = thread::spawn(move || {
            //Storage for our frames
            let mut frame_bytes = vec![0; 1024 * 32 * 8]; //Lets make this larger than usually needed, so multiple frames can find place here
            let find_magic_bytes = |start_offset, buffer: &[u8]| {
                const JPEG_MAGIC_NUMBER:[u8; 3] = [0xFF, 0xD8, 0xFF];
                for i in start_offset..(buffer.len() - 3){
                    if buffer[i..i+3] == JPEG_MAGIC_NUMBER{
                        return Some(i);
                    }
                }
                None
            };
            loop{
                let bytes_read = bytestream.read(&mut frame_bytes).unwrap_or(0);
                if bytes_read == 0{
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                if let Some(mut first_magic_number) = find_magic_bytes(0, &frame_bytes[0..bytes_read]){
                    eprintln!("first_magic_number found");
                    let mut buffer_end = bytes_read;
                    while buffer_end < frame_bytes.len() - (1024 * 32){ //Always leave space for a second frame
                        let bytes_read = bytestream.read(&mut frame_bytes[buffer_end..]).unwrap_or(0);
                        if bytes_read == 0{
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        if let Some(second_magic_number) = find_magic_bytes(buffer_end, &frame_bytes[0..(buffer_end + bytes_read)]){
                            //Found a jpeg frame!
                            WifiCam::decode_jpeg_frame(&frame_bytes[first_magic_number .. second_magic_number]);
                            //Copy rest of buffer into beginning and continue inner loop
                            let number_of_overhanging_bytes = (buffer_end + bytes_read) - second_magic_number;
                            for i in 0 .. number_of_overhanging_bytes{
                                frame_bytes[i] = frame_bytes[i + second_magic_number];
                            }
                            buffer_end = number_of_overhanging_bytes;
                            first_magic_number = 0;
                        }else{
                            //If we didn't find a second magic number we didn't load enough of the previous frame yet
                            //So we just move the end of the buffer forward by the required amount and load more bytes
                            buffer_end += bytes_read;
                        }
                    }
                    //If we end up here, we ran out of buffer space
                    eprintln!("Ran out of buffer space");
                    continue;
                }else{ 
                    //Magic number not in this packet, let's wait for the next one
                    continue;
                }
            }

            
        });

        jpeg_thread
    }

    fn decode_jpeg_frame(bytes: &[u8]){
        /*let mut decoder = Decoder::new(bytes);
        match decoder.decode(){
            Ok(pixels) => {
                let metadata = decoder.info().expect("Error reading metadata");
                eprintln!("Read frame: {:?}", metadata);
            },
            Err(e) => {
                eprintln!("Decoding error: {:?}", e);
            }
        }*/
        thread::sleep(Duration::from_millis(1));
        eprintln!("Frame {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
    }
}


#[derive(Debug, Clone)]
pub enum TcpMessage{
    Initialization,
    YellowWireHigh,
    YellowWireLow,
    Other(Vec<u8>)
}

impl From<&[u8]> for TcpMessage{
    fn from(bytes: &[u8]) -> Self {
        const INITIALIZATION: [u8; 8] = [0x0D, 0x02, 0xD0, 0xD0, 0x0E, 0x01, 0xAF, 0xE0];
        const YELLOW_WIRE_HIGH: [u8; 4] = [0x0B, 0x01, 0xC0, 0xB0];
        const YELLOW_WIRE_LOW: [u8; 4] = [0x0B, 0x00, 0xC0, 0xB0];

        if bytes == INITIALIZATION{
            TcpMessage::Initialization
        }else if bytes == YELLOW_WIRE_HIGH{
            TcpMessage::YellowWireHigh
        }else if bytes == YELLOW_WIRE_LOW{
            TcpMessage::YellowWireLow
        }else{
            TcpMessage::Other(Vec::from(bytes))
        }
    }
}