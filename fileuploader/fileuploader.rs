use std::fs::{metadata, File};
use std::io::prelude::*;
use std::io::ErrorKind;
use std::io::SeekFrom;
use std::net::TcpStream;
use std::thread;
use std::time;

const BUF_SIZE: usize = 1024;

pub struct FileUploader {
    host: String,
    port: u16,
}

impl FileUploader {
    pub fn new(host: String, port: u16) -> FileUploader {
        FileUploader { host, port }
    }

    pub fn upload(&self, filename: String) {
        let mut stream = self.connect();

        let mut bytes_acknowledged: u64 = 0;

        let file_size = metadata(&filename).expect("Failed to read file size").len();

        self.send_header(&mut stream, &filename, file_size, 0);

        let mut file = File::open(&filename).expect("Failed to open the file");

        let mut buf = [0 as u8; BUF_SIZE];
        let mut u64_buf = [0 as u8; 8];

        loop {
            stream
                .set_nonblocking(true)
                .expect("set_nonblocking call failed");
            match stream.read_exact(&mut u64_buf) {
                Ok(_) => {
                    bytes_acknowledged = u64::from_be_bytes(u64_buf);
                }
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {}
                    _ => eprintln!("WARNING: failed to read acknowledgement: {}", err),
                },
            }

            // FIXME: fix this blocking/not-blocking madness
            stream
                .set_nonblocking(false)
                .expect("set_nonblocking call failed");

            let n = file.read(&mut buf[..]).expect("Failed to read");
            match stream.write(&buf[..n]) {
                Ok(size) => {
                    if size == 0 {
                        while file_size != bytes_acknowledged {
                            match stream.read_exact(&mut u64_buf) {
                                Ok(_) => bytes_acknowledged = u64::from_be_bytes(u64_buf),
                                Err(err) => {
                                    eprintln!("WARNING: failed to read acknowledgement: {}", err)
                                }
                            }
                        }
                        break;
                    }
                }
                Err(e) => match e.kind() {
                    ErrorKind::ConnectionReset | ErrorKind::BrokenPipe => {
                        eprintln!("Connection reset");
                        stream = self.connect();
                        file.seek(SeekFrom::Start(bytes_acknowledged))
                            .expect("Failed to seek");
                        self.send_header(&mut stream, &filename, file_size, bytes_acknowledged);
                    }
                    _ => {
                        panic!("Unhandled error: {}", e);
                    }
                },
            }
        }
    }

    fn connect(&self) -> TcpStream {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream;

        loop {
            stream = TcpStream::connect(&addr);
            match stream {
                Err(err) => match err.kind() {
                    ErrorKind::ConnectionRefused => {
                        eprintln!("Connection refused. Retrying...");
                        thread::sleep(time::Duration::from_secs(1));
                    }
                    _ => panic!("Unhandled error: {}", err),
                },
                Ok(_) => break,
            }
        }

        eprintln!("Connection established");
        stream.unwrap()
    }

    fn send_header(
        &self,
        stream: &mut TcpStream,
        filename: &str,
        file_size: u64,
        file_offset: u64,
    ) {
        stream
            .write(&(filename.len() as u8).to_be_bytes())
            .expect("Failed to send filename length");

        stream
            .write(filename.as_bytes())
            .expect("Failed to send filename");

        stream
            .write(&file_size.to_be_bytes())
            .expect("Failed to send file size");

        stream
            .write(&file_offset.to_be_bytes())
            .expect("Failed to send file offset");
    }
}
