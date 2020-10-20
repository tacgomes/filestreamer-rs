use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

const BUF_SIZE: usize = 1024;
const MAX_BYTES_NOT_ACKNOWLEDGED: u64 = 1 * 1024 * 1024;
const POLLING_TIME: Duration = Duration::from_millis(200);

enum Action {
    Start = 0,
    Stop = 1,
    StopNow = 2,
}

impl Into<usize> for Action {
    fn into(self) -> usize {
        self as usize
    }
}

pub struct FileReceiver {
    port: u16,
    action: AtomicUsize,
}

impl FileReceiver {
    pub fn new(port: u16) -> FileReceiver {
        FileReceiver { port, action: AtomicUsize::new(Action::Start as usize) }
    }

    pub fn start(&self) {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(addr).expect("Failed to initiate server");
        listener.set_nonblocking(true).expect("Failed to non-blocking");
        self.action.store(Action::Start as usize, Ordering::Relaxed);

        for stream in listener.incoming() {
            match stream {
                Ok(s) => self.handle_connection(s),
                Err(err) => match err.kind() {
                    io::ErrorKind::WouldBlock => {
                        if self.action.load(Ordering::Relaxed) != Action::Start as usize {
                            break;
                        }
                        thread::sleep(POLLING_TIME);
                    },
                    _=> panic!("Encountered IO error: {}", err),
                }
            }
        }
    }

    pub fn stop(&self) {
        self.action.store(Action::Stop as usize, Ordering::Relaxed);
    }

    pub fn stop_now(&self) {
        self.action.store(Action::StopNow as usize, Ordering::Relaxed);
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let mut u8_buf = [0 as u8; 1];
        let mut u64_buf = [0 as u8; 8];

        stream
            .read_exact(&mut u8_buf)
            .expect("Failed to read filename length");
        let filename_len = u8::from_be_bytes(u8_buf);

        let mut filename_buf = vec![0 as u8; filename_len as usize];
        stream
            .read_exact(&mut filename_buf)
            .expect("Failed to read filename");
        let filename = String::from_utf8(filename_buf)
            .expect("Failed to construct filename string")
            + ".received";

        stream
            .read_exact(&mut u64_buf)
            .expect("Failed to read file size");
        let file_size = u64::from_be_bytes(u64_buf);

        stream
            .read_exact(&mut u64_buf)
            .expect("Failed to read file offset");
        let offset = u64::from_be_bytes(u64_buf);

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(Path::new(&filename).file_name().unwrap())
            .expect("Failed to open file");

        file.seek(SeekFrom::Start(offset)).expect("Failed to seek");

        let mut bytes_received: u64 = 0;
        let mut bytes_not_acknowledged: u64 = 0;
        let mut buf = [0 as u8; BUF_SIZE];

        while self.action.load(Ordering::Relaxed) != Action::StopNow as usize
                && match stream.read(&mut buf) {
            Ok(0) => {
                println!("File transfer completed");
                false
            }
            Ok(size) => {
                file.write_all(&buf[..size])
                    .expect("Failed to write to file");

                bytes_received += size as u64;
                bytes_not_acknowledged += size as u64;

                if bytes_not_acknowledged >= MAX_BYTES_NOT_ACKNOWLEDGED
                    || (offset + bytes_received) == file_size
                {
                    file.flush().expect("Failed to flush the file");

                    match stream.write(&(bytes_received + offset).to_be_bytes()) {
                        Err(err) => eprintln!("WARNING: failed to send acknowledgedment: {}", err),
                        Ok(_) => bytes_not_acknowledged = 0,
                    }
                }

                true
            }
            Err(err) => {
                eprintln!("Error reading from stream: {}", err);
                true
            }
        } {}
    }
}
