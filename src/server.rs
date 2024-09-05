use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt}; // Import SinkExt
use std::sync::Arc;
use std::collections::HashMap;

type Tx = broadcast::Sender<(String, String)>;
type Rx = broadcast::Receiver<(String, String)>;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080".to_string();
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    let (tx, _rx) = broadcast::channel(100);
    let clients = Arc::new(Mutex::new(HashMap::new()));

    println!("Server running on {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        let rx = tx.subscribe();
        let clients = clients.clone();
        tokio::spawn(handle_connection(stream, tx, rx, clients));
    }
}

async fn handle_connection(stream: TcpStream, tx: Tx, mut rx: Rx, clients: Arc<Mutex<HashMap<String, Tx>>>) {
    let ws_stream = accept_async(stream).await.expect("Error during the websocket handshake");
    let (mut write, mut read) = ws_stream.split();
    let nickname = Arc::new(Mutex::new(String::new()));
    let write = Arc::new(Mutex::new(write));

    // Task to handle incoming messages from the broadcast channel
    let nickname_clone = nickname.clone();
    let write_clone = write.clone();
    tokio::spawn(async move {
        while let Ok((sender, msg)) = rx.recv().await {
            let nickname = nickname_clone.lock().await;
            if sender != *nickname {
                let mut write = write_clone.lock().await;
                write.send(Message::Text(msg)).await.unwrap();
            }
        }
    });

    while let Some(Ok(msg)) = read.next().await {
        if let Message::Text(text) = msg {
            let mut nickname = nickname.lock().await;
            if nickname.is_empty() {
                *nickname = text.clone();
                clients.lock().await.insert(nickname.clone(), tx.clone());
                let client_list = clients.lock().await.keys().cloned().collect::<Vec<_>>().join(", ");
                let mut write = write.lock().await;
                write.send(Message::Text(format!("Connected players: {}", client_list))).await.unwrap();
                // Broadcast the join message to all clients
                tx.send((nickname.clone(), format!("{} has joined the game!", nickname))).unwrap();
            } else {
                tx.send((nickname.clone(), format!("{}: {}", nickname, text))).unwrap();
            }
        }
    }

    let nickname = nickname.lock().await;
    clients.lock().await.remove(&*nickname);
    tx.send((nickname.clone(), format!("{} has left the game!", nickname))).unwrap();
}
