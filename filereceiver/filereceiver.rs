use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

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
        let mut data = [0 as u8; 1];
        stream.read_exact(&mut data).expect("Failed to read filename length");
        let filename_length = u8::from_be_bytes(data);

        let mut filename_bytes = vec![0 as u8; filename_length as usize];
        stream.read_exact(&mut filename_bytes).expect("Failed to read filename bytes");
        let filename = String::from_utf8(filename_bytes)
            .expect("Failed to construct filename string");

        let mut filesize_data = [0 as u8; 8];
        stream.read_exact(&mut filesize_data).expect("Failed to read file size");
        let _filesize = u64::from_be_bytes(filesize_data);

        let path = std::path::Path::new(&filename);

        let mut file = std::fs::File::create(path.file_name().unwrap())
            .expect("Failed to open file");

        loop {
            let mut data = [0 as u8; 50];
            match stream.read(&mut data) {
                Ok(size) => {
                    if size == 0 {
                        println!("File transfer completed");
                        break;
                    } else {
                        file.write_all(&data[..size]).expect("Failed to write to file");
                    }
                },
                Err(err) => eprintln!("Error reading data: {}", err),
            }
        }
    }

}
