use std::io::prelude::*;
use std::fs::File;
use std::net::TcpStream;

pub struct FileUploader {
    host: String,
    port: u16,
}

impl FileUploader {
    pub fn new(host: String, port: u16) -> FileUploader {
        FileUploader {host, port}
    }

    pub fn upload(&self, filename: String) {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr).expect("Failed to connect");

        FileUploader::send_header(&mut stream, &filename);

        let mut f = File::open(&filename).expect("Failed to open the file");
        let mut buffer = [0; 1024];
        loop {
            let n = f.read(&mut buffer[..]).expect("Failed to read");
            stream.write(&buffer[..n]).expect("Failed to write to socket");
            if n == 0 {
                break;
            }
        }
    }

    fn send_header(stream: &mut TcpStream, filename: &str) {
        let meta = std::fs::metadata(&filename)
            .expect("Failed to read file size");

        stream.write(&(filename.len() as u8).to_be_bytes())
            .expect("Failed to send filename length");

        stream.write(filename.as_bytes())
            .expect("Failed to send filename");

        stream.write(&meta.len().to_be_bytes())
            .expect("Failed to send file size");
    }
}