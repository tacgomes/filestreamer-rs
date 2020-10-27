use std::path::PathBuf;

use structopt::StructOpt;

use file_uploader::FileUploader;

#[derive(Debug, StructOpt)]
#[structopt(name = "fileuploader", about = "Uploads a file")]
struct Cli {
    #[structopt(long)]
    host: String,

    #[structopt(long)]
    port: u16,

    #[structopt(long)]
    rate_limit: Option<u32>,

    #[structopt(parse(from_os_str), name = "FILE")]
    file_name: PathBuf,
}

fn main() {
    let args = Cli::from_args();

    let uploader = FileUploader::new(args.host, args.port, args.rate_limit);
    uploader.upload(args.file_name);
}
