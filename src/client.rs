use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use futures_util::{SinkExt, StreamExt};

#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:8080";
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    let stdin = io::stdin();
    let mut stdin_lines = BufReader::new(stdin).lines();

    println!("Enter your nickname:");
    if let Ok(Some(nickname)) = stdin_lines.next_line().await {
        write.send(Message::Text(nickname)).await.unwrap();
    }

    tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                println!("{}", text);
            }
        }
    });

    while let Ok(Some(line)) = stdin_lines.next_line().await {
        write.send(Message::Text(line)).await.unwrap();
    }
}
