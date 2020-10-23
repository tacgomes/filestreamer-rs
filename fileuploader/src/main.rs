use fileuploader::FileUploader;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fileuploader", about = "Uploads a file")]
struct Cli {
    #[structopt(long)]
    host: String,

    #[structopt(long)]
    port: u16,

    #[structopt(long)]
    rate_limit: Option<u32>,

    #[structopt(name = "FILE")]
    filename: String,
}

fn main() {
    let args = Cli::from_args();

    let rate_limit = match args.rate_limit {
        Some(val) => val,
        None => 0,
    };

    let uploader = FileUploader::new(args.host, args.port, rate_limit);
    uploader.upload(args.filename);
}
