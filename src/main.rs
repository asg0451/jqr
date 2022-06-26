use std::io::Read;

use anyhow::{bail, Result};
use clap::Parser;

use jqr::{parse_filter, Streamer};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Filter string
    #[clap()]
    filter: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    if let Err(std::env::VarError::NotPresent) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }

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

    let hack = r.chain("\nnull\n".as_bytes());

    let streamer = Streamer::new(hack);

    for v in streamer {
        let v = v?;
        let output = filter.apply(&v);

        if let Some(j) = output {
            println!("{}", j)
        }
    }

    Ok(())
}
