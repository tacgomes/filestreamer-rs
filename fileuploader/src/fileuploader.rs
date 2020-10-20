use std::fs::{metadata, File};
use std::io::prelude::*;
use std::io::ErrorKind;
use std::io::SeekFrom;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use crate::ratelimit::RateLimitedStream;

const BUF_SIZE: usize = 1024;

pub struct FileUploader {
    host: String,
    port: u16,
    rate_limit: u32,
}

impl FileUploader {
    pub fn new(host: String, port: u16, rate_limit: u32) -> FileUploader {
        FileUploader {
            host,
            port,
            rate_limit,
        }
    }

    pub fn upload(&self, filename: String) {
        let mut stream = RateLimitedStream::new(self.connect(), self.rate_limit as f64);

        let mut bytes_acknowledged = 0;
        let mut total_bytes_sent = 0;

        let file_size = metadata(&filename).expect("Failed to read file size").len();

        self.send_header(&mut stream, &filename, file_size, 0);

        let mut file = File::open(&filename).expect("Failed to open the file");

        let mut buf = [0 as u8; BUF_SIZE];
        let mut u64_buf = [0 as u8; 8];

        let now = Instant::now();

        loop {
            match stream.read_exact(&mut u64_buf) {
                Ok(_) => bytes_acknowledged = u64::from_be_bytes(u64_buf),
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {}
                    _ => eprintln!("WARNING: failed to read acknowledgement: {}", err),
                },
            }

            let bytes_read = file.read(&mut buf[..]).expect("Failed to read");
            if bytes_read == 0 {
                break;
            }

            let mut bytes_sent = 0;

            while bytes_sent != bytes_read {
                match stream.write(&buf[bytes_sent..bytes_read]) {
                    Ok(size) => {
                        bytes_sent += size;
                        total_bytes_sent += size;
                    }
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock => {}
                        ErrorKind::ConnectionReset | ErrorKind::BrokenPipe => {
                            eprintln!("Connection reset");
                            file.seek(SeekFrom::Start(bytes_acknowledged))
                                .expect("Failed to seek");
                            stream.update_stream(self.connect());
                            self.send_header(&mut stream, &filename, file_size, bytes_acknowledged);
                            break;
                        }
                        _ => panic!("Unhandled error: {}", e),
                    },
                }
            }
        }

        while bytes_acknowledged != file_size {
            match stream.read_exact(&mut u64_buf) {
                Ok(_) => bytes_acknowledged = u64::from_be_bytes(u64_buf),
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {}
                    _ => eprintln!("WARNING: failed to read acknowledgement: {}", err),
                },
            }
        }

        let secs = now.elapsed().as_secs_f64();
        let upload_speed = total_bytes_sent as f64 / secs;
        println!("Elapsed time: {:.2} seconds", secs);
        println!("Bytes transferred: {} bytes", total_bytes_sent);
        println!("Average upload speed: {} bytes/sec", upload_speed.round());
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
                        thread::sleep(Duration::from_secs(1));
                    }
                    _ => panic!("Unhandled error: {}", err),
                },
                Ok(_) => break,
            }
        }

        let stream = stream.unwrap();

        eprintln!("Connection established");

        stream
            .set_nonblocking(true)
            .expect("set_nonblocking call failed");

        stream
    }

    fn send_header(
        &self,
        stream: &mut RateLimitedStream<TcpStream>,
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
