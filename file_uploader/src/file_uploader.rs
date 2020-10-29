use std::cmp;
use std::fs::{metadata, File};
use std::io::{self, prelude::*, ErrorKind, SeekFrom};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::rate_limit::RateLimitedStream;

const BUF_SIZE: usize = 1024;

pub struct FileUploader {
    host: String,
    port: u16,
    rate_limit: Option<u32>,
}

impl FileUploader {
    pub fn new(host: String, port: u16, rate_limit: Option<u32>) -> FileUploader {
        FileUploader {
            host,
            port,
            rate_limit,
        }
    }

    pub fn upload(&self, file_name: impl AsRef<Path>) {
        let mut stream = RateLimitedStream::new(self.connect(), self.rate_limit);

        let mut bytes_acknowledged = 0;
        let mut total_bytes_sent = 0;

        let file_size = metadata(&file_name)
            .expect("Failed to read file size")
            .len();

        let mut file = File::open(&file_name).expect("Failed to open the file");

        let file_name = file_name.as_ref().to_string_lossy();

        self.send_header(&mut stream, &file_name, file_size, 0);

        let buf_size = match self.rate_limit {
            Some(val) => cmp::min(val as usize, BUF_SIZE),
            None => BUF_SIZE,
        };

        let mut buf = vec![0u8; buf_size];
        let mut u64_buf = [0u8; 8];

        let now = Instant::now();

        loop {
            match stream.read_exact(&mut u64_buf) {
                Ok(_) => {
                    bytes_acknowledged = u64::from_be_bytes(u64_buf);
                    self.update_progress_bar(bytes_acknowledged, file_size);
                }
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
                            self.send_header(
                                &mut stream,
                                &file_name,
                                file_size,
                                bytes_acknowledged,
                            );
                            break;
                        }
                        _ => panic!("Unhandled error: {}", e),
                    },
                }
            }
        }

        while bytes_acknowledged != file_size {
            match stream.read_exact(&mut u64_buf) {
                Ok(_) => {
                    bytes_acknowledged = u64::from_be_bytes(u64_buf);
                    self.update_progress_bar(bytes_acknowledged, file_size);
                }
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {}
                    _ => eprintln!("WARNING: failed to read acknowledgement: {}", err),
                },
            }
        }

        let secs = now.elapsed().as_secs_f64();
        let upload_speed = total_bytes_sent as f64 / secs;

        println!("File transfer completed");
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

        println!(
            "Connection established with: {}",
            stream.peer_addr().unwrap()
        );

        stream
            .set_nonblocking(true)
            .expect("set_nonblocking call failed");

        stream
    }

    fn send_header(
        &self,
        stream: &mut RateLimitedStream<TcpStream>,
        file_name: &str,
        file_size: u64,
        file_offset: u64,
    ) {
        stream
            .write(&(file_name.len() as u8).to_be_bytes())
            .expect("Failed to send file name length");

        stream
            .write(file_name.as_bytes())
            .expect("Failed to send file name");

        stream
            .write(&file_size.to_be_bytes())
            .expect("Failed to send file size");

        stream
            .write(&file_offset.to_be_bytes())
            .expect("Failed to send file offset");
    }

    fn update_progress_bar(&self, bytes_acknowledged: u64, file_size: u64) {
        let percentage = bytes_acknowledged as f64 / file_size as f64 * 100.0;
        let progress = "=".repeat(percentage as usize / 2);
        print!("\r[{:50}] {:.2}%", progress, percentage);
        io::stdout().flush().unwrap();

        if bytes_acknowledged == file_size {
            print!("\x1B[2K\r"); // Clear current line
            io::stdout().flush().unwrap();
        }
    }
}
