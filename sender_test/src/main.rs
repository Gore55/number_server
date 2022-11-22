use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, BufWriter};
use rand::thread_rng;
use rand::prelude::*;

#[tokio::main]
async fn main() {

    let mut rng = thread_rng();
    
    let stream = TcpStream::connect("127.0.0.1:4000").await.expect("Failed to connect");
    let mut writer = BufWriter::new(stream);

    loop {
        let stringed = format!("{}\n", rng.gen_range(0..999999999));
        let _ = &writer.write(&mut stringed.as_bytes()).await.expect("Failed to write");
        writer.flush().await.expect("Failed to flush");
    }

}