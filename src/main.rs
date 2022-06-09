use anyhow::Result;
use jqr::{parse_filter, streamer::Streamer};

fn main() -> Result<()> {
    let filter_str = " .hello.man ";
    let filter = parse_filter(filter_str)?;

    let stdin = std::io::stdin();
    let r = stdin.lock();
    // let mut r = r#"{"hello": {"man": 42}}\n"#.as_bytes();

    let streamer = Streamer::new(r);

    for v in streamer {
        dbg!(&v);
        let v = v?;
        let output = filter.apply(&v);
        dbg!(&output);
    }

    Ok(())
}
