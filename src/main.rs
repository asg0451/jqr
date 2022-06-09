use anyhow::Result;
use jqr::streamer::Streamer;

fn main() -> Result<()> {
    // let stdin = std::io::stdin();
    // let r = stdin.lock();
    let mut r = r#"{"hello": 42}\n"#.as_bytes();
    let streamer = Streamer::new(&mut r);

    for v in streamer {
        dbg!(&v);
        let _v = v?;
    }

    Ok(())
}
