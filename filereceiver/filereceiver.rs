use std::io::prelude::*;
use std::io::SeekFrom;
use std::net::{TcpListener, TcpStream};
use std::path::Path;

const BUF_SIZE : usize = 1024;
const MAX_BYTES_NOT_ACKNOWLEDGED : u64 = 1 * 1024 * 1024;

pub struct FileReceiver {
    port: u16
}

impl FileReceiver {
    pub fn new(port: u16) -> FileReceiver {
        FileReceiver { port }
    }

    pub fn start(&self) {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(addr).expect("Failed to initiate server");

        for stream in listener.incoming() {
            FileReceiver::handle_connection(
                stream.expect("Failed to create connection"));
        }
    }

    fn handle_connection(mut stream: TcpStream) {
        let mut u8_buf = [0 as u8; 1];
        let mut u64_buf = [0 as u8; 8];

        stream.read_exact(&mut u8_buf).expect("Failed to read filename length");
        let filename_len = u8::from_be_bytes(u8_buf);

        let mut filename_buf = vec![0 as u8; filename_len as usize];
        stream.read_exact(&mut filename_buf).expect("Failed to read filename");
        let filename = String::from_utf8(filename_buf)
            .expect("Failed to construct filename string");

        stream.read_exact(&mut u64_buf).expect("Failed to read file size");
        let file_size = u64::from_be_bytes(u64_buf);

        stream.read_exact(&mut u64_buf).expect("Failed to read file offset");
        let offset = u64::from_be_bytes(u64_buf);

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(Path::new(&filename).file_name().unwrap())
            .expect("Failed to open file");

        file.seek(SeekFrom::Start(offset)).expect("Failed to seek");

        let mut bytes_received : u64 = 0;
        let mut bytes_not_acknowledged : u64 = 0;
        let mut buf = [0 as u8; BUF_SIZE];

        loop {
            match stream.read(&mut buf) {
                Ok(size) => {
                    if size == 0 {
                        println!("File transfer completed");
                        break;
                    } else {
                        file.write_all(&buf[..size]).expect("Failed to write to file");

                        bytes_received += size as u64;
                        bytes_not_acknowledged += size as u64;

                        if bytes_not_acknowledged >= MAX_BYTES_NOT_ACKNOWLEDGED
                                || (offset + bytes_received) == file_size {
                            file.flush().expect("Failed to flush the file");
                            match stream.write(&(bytes_received + offset).to_be_bytes()) {
                                Err(err) => eprintln!("WARNING: failed to send acknowledgedment: {}", err),
                                Ok(_) => bytes_not_acknowledged = 0,
                            }
                        }
                    }
                },
                Err(err) => eprintln!("Error reading data: {}", err),
            }
        }
    }
}
