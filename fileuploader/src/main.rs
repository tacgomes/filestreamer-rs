use std::env;
use fileuploader::FileUploader;

fn show_usage(progname: &str) {
    eprintln!(
        "Usage: {} [Options...] FILE

Options:
\t--host HOST\t\tSpecify the server IP address
\t--port HOST\t\tSpecify the server port
\t--limit-rate RATE\tLimit upload speed (bytes/second)
\t-h,--help\t\tShow usage",
        progname
    );
}

fn main() {
    let mut host = String::new();
    let mut port: u16 = 0;
    let mut rate_limit: u32 = 0;
    let mut filename = String::new();

    let mut i = 1;
    let args: Vec<String> = env::args().collect();

    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                if i + 1 >= args.len() {
                    show_usage(&args[0]);
                }
                host = args[i + 1].clone();
                i += 1;
            }
            "--port" => {
                if i + 1 >= args.len() {
                    show_usage(&args[0]);
                }
                port = args[i + 1].parse::<u16>().unwrap();
                i += 1;
            }
            "--limit-rate" => {
                if i + 1 >= args.len() {
                    show_usage(&args[0]);
                }
                rate_limit = args[i + 1].parse::<u32>().unwrap();
                i += 1;
            }
            "--help" | "-h" => {
                show_usage(&args[0]);
                std::process::exit(0);
            }
            _ => filename = args[i].clone(),
        }
        i += 1;
    }

    if host.is_empty() || port == 0 || filename.is_empty() {
        eprintln!("Error: missing arguments\n");
        show_usage(&args[0]);
        std::process::exit(1);
    }

    let uploader = FileUploader::new(host, port, rate_limit);
    uploader.upload(filename);
}
