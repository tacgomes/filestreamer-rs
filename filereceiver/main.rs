use std::env;

mod filereceiver;

fn show_usage(progname: &str) {
    eprintln!(
"Usage: {} [Options...] PORT

Options:
\t-h,--help\t\tShow usage",
        progname
    );
}

fn main() {
    let mut port : u16 = 0;

    let mut i = 1;
    let args : Vec<String> = env::args().collect();

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                show_usage(&args[0]);
                std::process::exit(0);
            },
            _ => port = args[i].parse::<u16>().unwrap()

        }
        i += 1;
    }

    if port == 0 {
        eprintln!("Error: missing arguments\n");
        show_usage(&args[0]);
        std::process::exit(1);
    }

    let receiver = filereceiver::FileReceiver::new(port);
    receiver.start();
}