use std::io::Read;

use anyhow::{bail, Result};
use clap::Parser;
use tracing::info;

use jqr::{parse_filter, Streamer};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Filter string
    #[clap()]
    filter: String,

    /// Input path (empty for stdin)
    #[clap()]
    input_file: Option<std::path::PathBuf>,
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

    info!("filter: {:?}", filter);

    let s = std::io::stdin();
    let reader: Box<dyn Read> = match args.input_file {
        // TODO: support Some("-")
        None => Box::new(s.lock()),
        Some(p) => Box::new(std::fs::File::open(p)?),
    };

    let hack = reader.chain("\nnull\n".as_bytes());

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
