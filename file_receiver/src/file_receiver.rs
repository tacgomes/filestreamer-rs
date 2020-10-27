use std::io::{self, prelude::*, SeekFrom};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

const BUF_SIZE: usize = 1024;
const MAX_BYTES_NOT_ACKNOWLEDGED: u64 = 1024 * 1024;
const POLLING_TIME: Duration = Duration::from_millis(200);

#[derive(PartialEq)]
enum Command {
    Run = 0,
    Stop = 1,
    StopNow = 2,
}

impl From<usize> for Command {
    fn from(value: usize) -> Self {
        match value {
            0 => Command::Run,
            1 => Command::Stop,
            2 => Command::StopNow,
            _ => unreachable!(),
        }
    }
}

pub struct FileReceiver {
    port: u16,
    command: AtomicUsize,
}

impl FileReceiver {
    pub fn new(port: u16) -> FileReceiver {
        FileReceiver {
            port,
            command: AtomicUsize::new(Command::Stop as usize),
        }
    }

    pub fn start(&self) {
        println!("Listening for file upload requests in port: {}", self.port);

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(addr).expect("Failed to initiate server");
        listener
            .set_nonblocking(true)
            .expect("Failed to non-blocking");
        self.set_command(Command::Run);

        for stream in listener.incoming() {
            match stream {
                Ok(s) => self.handle_connection(s),
                Err(err) => match err.kind() {
                    io::ErrorKind::WouldBlock => {
                        if self.get_command() != Command::Run {
                            break;
                        }
                        thread::sleep(POLLING_TIME);
                    }
                    _ => panic!("Encountered IO error: {}", err),
                },
            }
        }
    }

    pub fn stop(&self) {
        self.set_command(Command::Stop);
    }

    pub fn stop_now(&self) {
        self.set_command(Command::StopNow);
    }

    fn get_command(&self) -> Command {
        Command::from(self.command.load(Ordering::Relaxed))
    }

    fn set_command(&self, command: Command) {
        self.command.store(command as usize, Ordering::Relaxed);
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        println!("Handling new request from: {}", stream.peer_addr().unwrap());

        let mut u8_buf = [0u8; 1];
        let mut u64_buf = [0u8; 8];

        stream
            .read_exact(&mut u8_buf)
            .expect("Failed to read file name length");
        let file_name_len = u8::from_be_bytes(u8_buf);

        let mut file_name_buf = vec![0u8; file_name_len as usize];
        stream
            .read_exact(&mut file_name_buf)
            .expect("Failed to read file_name");
        let file_name =
            String::from_utf8(file_name_buf).expect("Failed to construct file name string");

        stream
            .read_exact(&mut u64_buf)
            .expect("Failed to read file size");
        let file_size = u64::from_be_bytes(u64_buf);

        stream
            .read_exact(&mut u64_buf)
            .expect("Failed to read file offset");
        let offset = u64::from_be_bytes(u64_buf);

        println!(
            "Receiving file: {} (size={}, offset={})",
            file_name, file_size, offset
        );

        let file_name = file_name + ".received";

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(Path::new(&file_name).file_name().unwrap())
            .expect("Failed to open file");

        file.seek(SeekFrom::Start(offset)).expect("Failed to seek");

        let mut bytes_received: u64 = 0;
        let mut bytes_not_acknowledged: u64 = 0;
        let mut buf = [0u8; BUF_SIZE];

        while self.get_command() != Command::StopNow
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
                            Err(err) => {
                                eprintln!("WARNING: failed to send acknowledgedment: {}", err)
                            }
                            Ok(_) => bytes_not_acknowledged = 0,
                        }
                    }

                    true
                }
                Err(err) => {
                    eprintln!("Error reading from stream: {}", err);
                    true
                }
            }
        {}
    }
}
