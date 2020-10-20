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
        let _stream = TcpStream::connect(&addr).expect("Failed to connect");

        let mut f = File::open(filename).expect("Failed to open the file");
        let mut buffer = [0; 1024];
        loop {
            let n = f.read(&mut buffer[..]).expect("Failed to read");
            if n == 0 {
                break;
            }
        }
    }
}