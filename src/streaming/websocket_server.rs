use crate::core::events::Event;
use futures_util::StreamExt;
use std::sync::mpsc::Sender;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

pub fn start_websocket_server(event_sender: Sender<Event>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let addr = "127.0.0.1:8080";
            let listener = TcpListener::bind(&addr).await.expect("Can't listen");
            println!("WebSocket server listening on: {}", addr);

            while let Ok((stream, _)) = listener.accept().await {
                let sender = event_sender.clone();
                tokio::spawn(async move {
                    let ws_stream = accept_async(stream)
                        .await
                        .expect("Error during the websocket handshake occurred");

                    let (_, mut read) = ws_stream.split();

                    while let Some(message) = read.next().await {
                        match message {
                            Ok(Message::Text(text)) => {
                                let _ = sender.send(Event::WebSocketMessage(text.to_string()));
                            }
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }
                });
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::Event;
    use futures_util::SinkExt;
    use std::sync::mpsc::channel;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::protocol::Message;

    #[tokio::test]
    async fn test_websocket_server() {
        let (tx, rx) = channel();

        // Starting the Server on a different Port for testing to avoid Conflict if 8080 is Used
        let event_sender = tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let addr = "127.0.0.1:8081";
                let listener = TcpListener::bind(&addr).await.expect("Can't listen");

                if let Ok((stream, _)) = listener.accept().await {
                    let ws_stream = accept_async(stream)
                        .await
                        .expect("Error during the websocket handshake occurred");

                    let (_, mut read) = ws_stream.split();

                    while let Some(message) = read.next().await {
                        match message {
                            Ok(Message::Text(text)) => {
                                let _ = event_sender.send(Event::WebSocketMessage(text.to_string()));
                            }
                            _ => break,
                        }
                    }
                }
            });
        });

        // Giving the Server a Moment to start
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let (mut ws_stream, _) = connect_async("ws://127.0.0.1:8081")
            .await
            .expect("Failed to connect");

        let test_msg = "Hello WebSocket".to_string();
        ws_stream
            .send(Message::Text(test_msg.clone().into()))
            .await
            .expect("Failed to send message");

        // Checking if Message Received in the Channel
        let received = rx.recv_timeout(std::time::Duration::from_secs(2)).expect("Failed to receive message");
        if let Event::WebSocketMessage(msg) = received {
            assert_eq!(msg, test_msg);
        } else {
            panic!("Received wrong event type");
        }
    }
}
