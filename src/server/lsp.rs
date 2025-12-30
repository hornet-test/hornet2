use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tower_lsp::{LspService, Server};

use crate::lsp::ArazzoLanguageServer;
use crate::server::state::AppState;

/// Handler for the LSP WebSocket route
pub async fn lsp_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    println!("LSP: New WebSocket connection");
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a duplex stream to bridge between the "server's stdin/stdout" and our WebSocket
    // server_read/server_write -> The side connected to the Language Server
    // client_read/client_write -> The side connected to the WebSocket (via pumping tasks)
    let (client_read, client_write) = tokio::io::duplex(1024 * 1024);
    let (server_read, mut server_write) = tokio::io::duplex(1024 * 1024);

    // Task to pump from WebSocket to Server Input (ADD HEADERS)
    tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("LSP IN: {}", text);
                    // LSP requires Content-Length header
                    let payload = format!("Content-Length: {}\r\n\r\n{}", text.len(), text);
                    if let Err(e) = server_write.write_all(payload.as_bytes()).await {
                        eprintln!("Error writing to LSP server: {}", e);
                        break;
                    }
                }
                Ok(Message::Binary(_)) => {
                    println!("LSP IN: Binary (ignored)");
                }
                Ok(Message::Close(_)) => {
                    println!("LSP IN: Close");
                    break;
                }
                Err(e) => {
                    eprintln!("LSP IN Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Task to pump from Server Output to WebSocket (STRIP HEADERS)
    let mut client_read = client_read;
    tokio::spawn(async move {
        let mut reader = BufReader::new(&mut client_read);
        let mut buffer = String::new();

        loop {
            // 1. Read Headers
            let mut content_length = 0;
            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => return, // EOF
                    Ok(_) => {
                        let line = buffer.trim();
                        if line.is_empty() {
                            // Empty line marks end of headers
                            break;
                        }
                        if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                            if let Ok(len) = len_str.parse::<usize>() {
                                content_length = len;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from LSP server: {}", e);
                        return;
                    }
                }
            }

            // 2. Read Body
            if content_length > 0 {
                let mut body = vec![0u8; content_length];
                match reader.read_exact(&mut body).await {
                    Ok(_) => {
                        // Send as Text
                        let text = String::from_utf8_lossy(&body);
                        println!("LSP OUT: {}", text);
                        if let Err(e) = ws_sender.send(Message::Text(text.to_string().into())).await
                        {
                            eprintln!("Error sending to WS: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading body from LSP server: {}", e);
                        break;
                    }
                }
            }
        }
    });

    let stdin = server_read;
    let stdout = client_write;

    let (service, socket) =
        LspService::new(|client| ArazzoLanguageServer::new(client, state.root_dir));

    Server::new(stdin, stdout, socket).serve(service).await;
}
