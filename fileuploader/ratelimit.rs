use std::io;
use std::io::prelude::*;
use std::time::{Duration, Instant};

pub struct RateLimitedStream<T> {
    stream: T,
    token_rate: f64,
    available_tokens: f64,
    last_updated: Instant,
}

impl<T: Read> Read for RateLimitedStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl<T: Write> RateLimitedStream<T> {
    pub fn new(stream: T, token_rate: f64) -> RateLimitedStream<T> {
        RateLimitedStream {
            token_rate,
            available_tokens: 0.0,
            last_updated: Instant::now(),
            stream,
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.token_rate != 0.0 {
            self.reserve(buf.len());
        }
        self.stream.write(buf)
    }

    pub fn update_stream(&mut self, stream: T) {
        self.stream = stream;
    }

    fn reserve(&mut self, required_tokens: usize) {
        let required_tokens = required_tokens as f64;

        if required_tokens > self.token_rate {
            panic!("Requested number of tokens can not exceed capacity");
        }

        self.sync();

        if self.available_tokens < required_tokens {
            let missing_tokens = required_tokens - self.available_tokens;
            let waiting_time = missing_tokens.ceil() / self.token_rate;
            std::thread::sleep(Duration::from_secs_f64(waiting_time));
            self.sync();
        }

        assert!(self.available_tokens >= required_tokens);
        self.available_tokens -= required_tokens;
    }

    fn sync(&mut self) {
        let current_time = Instant::now();

        let time_elapsed = current_time.duration_since(self.last_updated).as_nanos();

        self.available_tokens += f64::min(
            time_elapsed as f64 * self.token_rate / 1_000_000_000.0,
            self.token_rate,
        );

        self.last_updated = current_time;
    }
}
