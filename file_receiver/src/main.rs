use structopt::StructOpt;

use file_receiver::FileReceiver;

#[derive(Debug, StructOpt)]
#[structopt(name = "filereceiver", about = "Receives a file")]
struct Cli {
    port: u16,
}

fn main() {
    let args = Cli::from_args();

    let receiver = FileReceiver::new(args.port);
    receiver.start();
}
