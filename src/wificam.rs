use std::{collections::VecDeque, net::{TcpStream, UdpSocket}, thread::{self, JoinHandle}, time::{Duration, SystemTime, UNIX_EPOCH}, u16};
use std::sync::mpsc::{channel, Sender,Receiver};
use arc_swap::ArcSwap;
use jpeg_decoder::Decoder;
use ringbuf::{RingBuffer, Producer, Consumer};
use smallvec::SmallVec;
use std::io::prelude::*;
use std::sync::Arc;

pub struct WifiCam{
    udp_thread: JoinHandle<()>,
    tcp_thread: JoinHandle<()>,
    //jpeg_thread: JoinHandle<()>,
    pub tcp_messages: Receiver<TcpMessage>,
    pub last_frame: Arc<ArcSwap<Vec<u8>>>
}

impl WifiCam{
    pub fn run(self){
        self.udp_thread.join();
        self.tcp_thread.join();
        //self.jpeg_thread.join();
    }

    pub fn new() -> WifiCam{
        let last_frame = Arc::new(ArcSwap::from_pointee(Vec::new()));
        let (udp_thread, consumer) = WifiCam::start_udp_receiver(last_frame.clone());
        let (tcp_thread, tcp_messages) = WifiCam::send_init_sequence();
        //let (last_frame, jpeg_thread) = WifiCam::start_jpeg_thread(consumer);
        WifiCam{
            udp_thread,
            tcp_thread,
            tcp_messages,
            //jpeg_thread,
            last_frame
        }
    }

    fn start_udp_receiver(last_frame: Arc<ArcSwap<Vec<u8>>>) -> (JoinHandle<()>, Consumer<u8>){
        //Create ringbuffer to communicate with decoder
        let ringbuffer = RingBuffer::new(1024 * 1024);
        let (mut producer, consumer) = ringbuffer.split();
        let mut raw_jpeg_buffer = vec![0u8; 128 * 1024];
        let mut raw_jpeg_buffer_length = 0;
        let mut frame_reception_state = FrameReceptionState::WaitingForFrameStart;
        //Spawn udp receiver thread
        let udp_thread = thread::spawn(move || {
            //Bind to address
            let socket = UdpSocket::bind("192.168.1.2:5555").expect("Error binding to UDP socket");
            //Receive packets and write to ringbuffer
            let mut buf = vec![0; 1024 * 32];
            loop{
                let (length, src) = socket.recv_from(&mut buf).expect("Error receiving UDP data");
                if length < 9{
                    eprintln!("Received less than 9 bytes (no header)");
                    continue;
                }
                let header = UDPFrameHeader::from(&buf[0..]);
                let data = &buf[9..length];
                if data.len() + 9 != header.packet_length as usize{
                    eprintln!("Received incomplete packet! {} != {}", data.len(), header.packet_length - 9);
                }

                match frame_reception_state{
                    FrameReceptionState::WaitingForFrameStart => {
                        if header.sub_packet_number != 0{ //Received packet from another image, discard
                            continue;
                        }else{
                            (&mut raw_jpeg_buffer[0..]).write_all(data).expect("Packet way longer than expected (memory corruption??)");
                            //eprintln!("initial bytes: {:?}", &raw_jpeg_buffer[0..5]);
                            //eprintln!("inital data bytes: {:?}", &data[0..5]);
                            raw_jpeg_buffer_length = data.len();
                            if header.total_sub_packets == 1{
                                if let Some(frame) = WifiCam::decode_jpeg_frame(&raw_jpeg_buffer){
                                    last_frame.store(Arc::new(frame));
                                }
                                frame_reception_state = FrameReceptionState::WaitingForFrameStart;
                                continue;
                            }else{
                                frame_reception_state = FrameReceptionState::AssemblingSubpackets{
                                    received: 1,
                                    required: header.total_sub_packets,
                                    last_packet_number: header.packet_number,
                                    frame_number: header.frame_number
                                }
                            }
                        }
                    },
                    FrameReceptionState::AssemblingSubpackets { received, required, last_packet_number, frame_number } => {
                        if header.packet_number != (last_packet_number.wrapping_add(1)){
                            eprintln!("Received packet out of order!");
                            frame_reception_state = FrameReceptionState::WaitingForFrameStart;
                            continue;
                        }else{
                            if header.frame_number != frame_number{
                                eprintln!("Received subpacket for other frame!");
                                frame_reception_state = FrameReceptionState::WaitingForFrameStart;
                                continue;
                            }else{
                                if header.sub_packet_number != received{
                                    eprintln!("Received subpacket out of order! Got {} expected {}", header.sub_packet_number, received.wrapping_add(1));
                                    frame_reception_state = FrameReceptionState::WaitingForFrameStart;
                                    continue;
                                }else{
                                    (&mut raw_jpeg_buffer[raw_jpeg_buffer_length..]).write_all(&data).expect(&format!("Packet way longer than expected (memory corruption??) subpacket_count={}, total_buffer_length={}, buffer_rest_length={}, data_length={}", required, raw_jpeg_buffer.len(), raw_jpeg_buffer[raw_jpeg_buffer_length..].len(), data.len()));
                                    raw_jpeg_buffer_length += data.len();
                                    if received + 1 == required{
                                        if let Some(frame) = WifiCam::decode_jpeg_frame(&raw_jpeg_buffer){
                                            last_frame.store(Arc::new(frame));
                                        }
                                        frame_reception_state = FrameReceptionState::WaitingForFrameStart;
                                        continue;
                                    }else{
                                        frame_reception_state = FrameReceptionState::AssemblingSubpackets{
                                            received: received.wrapping_add(1),
                                            required: header.total_sub_packets,
                                            last_packet_number: header.packet_number,
                                            frame_number: header.frame_number
                                        }
                                    }
                                }
                            }
                        }
                    }
                }


                //producer.write_all(&buf[0..length]).expect("Error writing to ringbuffer");
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
            const INIT_SEQUENCE: [u8; 20] = [
                0x01, 0x01, 0x02, 0x10,
                0x02, 0x01, 0x03, 0x20, 0x02, 0x01, 0x03, 0x20,
                0x0e, 0x01, 0xaf, 0xe0, 0x24, 0x01, 0xc0, 0x42
            ];
            const KEEPALIVE_SEQUENCE: [u8; 4] = [
                0x0e, 0x01, 0xaf, 0xe0
            ];
            const KEEPALIVE_INTERVAL_MS: u128 = 1000;
            //Connect to tcp port
            let mut stream = TcpStream::connect("192.168.1.1:5252").expect("Error binding to TCP socket");
            stream.set_read_timeout(Some(Duration::from_millis(100))).expect("Error setting receive timeout");
            //Write magic sequence
            stream.write_all(&INIT_SEQUENCE).expect("Error sending TCP data");
            //Start reading
            let mut last_keepalive = SystemTime::now();
            loop {
                 //Send keepalive sequence if more than one second since last keepalive
                 if SystemTime::now().duration_since(last_keepalive).map(|duration| duration.as_millis() > KEEPALIVE_INTERVAL_MS).unwrap_or(false){
                    //eprintln!("Sending keepalive");
                    stream.write_all(&KEEPALIVE_SEQUENCE).expect("Error sending keepalive sequence");
                    last_keepalive = SystemTime::now();
                }
                //Read answer
                let mut buf = [0;256];
                let length_read =
                match stream.read(&mut buf){
                    Ok(number) => number,
                    Err(e) => {
                        let kind = e.kind();
                        match kind{
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut=> {
                                //eprintln!("Read timeout");
                            },
                            _ => {
                                eprintln!("Error reading tcp data: {:?}",e);
                            }
                        }
                        continue;
                    }
                };
                //After receiving, transform message into TcpMessage and pass on
                let message: TcpMessage = buf[0..length_read].into();
                eprintln!("Received {} bytes from remote TCP (Message = {:?})",length_read, message);
                sender.send(message).expect("Error sending TCP message to queue");
            }
        });
        
        (tcp_thread, receiver)
    }

    fn start_jpeg_thread(mut bytestream: Consumer<u8>) -> (Arc<ArcSwap<Vec<u8>>>, JoinHandle<()>){
        let last_frame = Arc::new(ArcSwap::from_pointee(Vec::new()));
        let frame_reference = last_frame.clone();
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
                            if let Some(frame) = WifiCam::decode_jpeg_frame(&frame_bytes[first_magic_number .. second_magic_number]){
                                frame_reference.store(Arc::new(frame));
                            }
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

        (last_frame, jpeg_thread)
    }

    fn decode_jpeg_frame(bytes: &[u8]) -> Option<Vec<u8>>{
        //eprintln!("Decoding frame starting with {:?}", &bytes[0..5]);
        let mut decoder = Decoder::new(bytes);
        match decoder.decode(){
            Ok(pixels) => {
                let metadata = decoder.info().expect("Error reading metadata");
                if metadata.width != 1280{
                    eprintln!("Width wrong");
                    return None;
                }
                if metadata.height != 720{
                    eprintln!("Height wrong");
                    return None;
                }
                if pixels.len() != 1280 * 720 * 3{
                    eprintln!("Pixel length wrong");
                    return None;
                }
                //eprintln!("Read frame: {:?}", metadata);
                Some(pixels)
            },
            Err(e) => {
                eprintln!("Decoding error: {:?}", e);
                None
            }
        }
    }
}


pub enum FrameReceptionState{
    WaitingForFrameStart,
    AssemblingSubpackets{
        received: u8,
        required: u8,
        frame_number: u8,
        last_packet_number: u16
    },
}

pub struct UDPFrameHeader{
    packet_number: u16,
    frame_number: u8,
    sub_packet_number: u8,
    total_sub_packets: u8,
    packet_length: u32
}

impl From<&[u8]> for UDPFrameHeader{
    fn from(bytes: &[u8]) -> Self {
        UDPFrameHeader{
            packet_number: u16::from_be_bytes([bytes[0], bytes[1]]),
            frame_number: bytes[2],
            sub_packet_number: bytes[3],
            total_sub_packets: bytes[4],
            packet_length: u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]])
        }
    }
}


#[derive(Debug, Clone)]
pub enum TcpMessage{
    Initialization,
    YellowWireHigh,
    YellowWireLow,
    KeepaliveAcknowledgement,
    Other(Vec<u8>)
}

impl From<&[u8]> for TcpMessage{
    fn from(bytes: &[u8]) -> Self {
        const INITIALIZATION: [u8; 8] = [0x0D, 0x02, 0xD0, 0xD0, 0x0E, 0x01, 0xAF, 0xE0];
        const YELLOW_WIRE_HIGH: [u8; 4] = [0x0B, 0x01, 0xC0, 0xB0];
        const YELLOW_WIRE_LOW: [u8; 4] = [0x0B, 0x00, 0xC0, 0xB0];
        const KEEPALIVE_ACKNOWLEDGEMENT: [u8; 4] = [0x0e, 0x01, 0xaf, 0xe0];

        if bytes == INITIALIZATION{
            TcpMessage::Initialization
        }else if bytes == YELLOW_WIRE_HIGH{
            TcpMessage::YellowWireHigh
        }else if bytes == YELLOW_WIRE_LOW{
            TcpMessage::YellowWireLow
        }else if bytes == KEEPALIVE_ACKNOWLEDGEMENT{
            TcpMessage::KeepaliveAcknowledgement
        }else{
            TcpMessage::Other(Vec::from(bytes))
        }
    }
}