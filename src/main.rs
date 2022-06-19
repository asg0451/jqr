use anyhow::{bail, Result};
use clap::Parser;

use jqr::{parse_filter, streamer::Streamer};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Filter string
    #[clap(short, long)]
    filter: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let filter = match parse_filter(&args.filter) {
        Ok(f) => f,
        Err(e) => {
            let msg = nom::error::convert_error::<&str>(&args.filter, e.clone());
            bail!("failed to parse filter: {}", msg);
        }
    };

    let stdin = std::io::stdin();
    let r = stdin.lock();

    let streamer = Streamer::new(r);

    for v in streamer {
        dbg!(&v);
        let v = v?;
        let output = filter.apply(&v);
        dbg!(&output);
    }

    Ok(())
}
