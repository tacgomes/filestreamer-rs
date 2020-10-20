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
            FileReceiver::_handle_client(stream.expect("Failed to create stream"));
        }
    }

    fn _handle_client(_stream: TcpStream) {
    }

}
