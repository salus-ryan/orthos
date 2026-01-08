use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tracing::{error, info, warn};

type DaemonHandle = Arc<Mutex<Child>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("orthos_bridge=info".parse().unwrap()),
        )
        .init();

    info!("ORTHOS Bridge starting...");

    let tcp_port: u16 = env::var("TCP_PORT")
        .unwrap_or_else(|_| "7777".to_string())
        .parse()
        .unwrap_or(7777);

    let ws_port: u16 = env::var("WS_PORT")
        .unwrap_or_else(|_| "7778".to_string())
        .parse()
        .unwrap_or(7778);

    // Start TCP and WebSocket servers
    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", tcp_port)).await?;
    let ws_listener = TcpListener::bind(format!("0.0.0.0:{}", ws_port)).await?;

    info!("TCP server listening on port {}", tcp_port);
    info!("WebSocket server listening on port {}", ws_port);

    // Handle connections
    tokio::select! {
        _ = handle_tcp_connections(tcp_listener) => {},
        _ = handle_ws_connections(ws_listener) => {},
    }

    Ok(())
}

async fn handle_tcp_connections(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("TCP connection from {}", addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_client(stream).await {
                        error!("TCP client error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("TCP accept error: {}", e);
            }
        }
    }
}

async fn handle_tcp_client(stream: TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(reader);

    // Spawn daemon for this connection
    let mut daemon = spawn_daemon()?;
    let daemon_stdin = daemon.stdin.take().unwrap();
    let daemon_stdout = daemon.stdout.take().unwrap();

    let daemon_stdin = Arc::new(Mutex::new(daemon_stdin));
    let daemon_stdout = Arc::new(Mutex::new(BufReader::new(daemon_stdout)));

    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        // Forward to daemon
        {
            let mut stdin = daemon_stdin.lock().await;
            stdin.write_all(line.as_bytes())?;
            stdin.flush()?;
        }

        // Read response from daemon
        let response = {
            let mut stdout = daemon_stdout.lock().await;
            let mut resp = String::new();
            stdout.read_line(&mut resp)?;
            resp
        };

        // Send back to client
        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;
    }

    // Cleanup
    let _ = daemon.kill();
    Ok(())
}

async fn handle_ws_connections(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("WebSocket connection from {}", addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_ws_client(stream).await {
                        error!("WebSocket client error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("WebSocket accept error: {}", e);
            }
        }
    }
}

async fn handle_ws_client(stream: TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Spawn daemon for this connection
    let mut daemon = spawn_daemon()?;
    let daemon_stdin = daemon.stdin.take().unwrap();
    let daemon_stdout = daemon.stdout.take().unwrap();

    let daemon_stdin = Arc::new(Mutex::new(daemon_stdin));
    let daemon_stdout = Arc::new(Mutex::new(BufReader::new(daemon_stdout)));

    while let Some(msg) = ws_receiver.next().await {
        let msg = msg?;
        if msg.is_text() {
            let text = msg.to_text()?;

            // Forward to daemon
            {
                let mut stdin = daemon_stdin.lock().await;
                writeln!(stdin, "{}", text)?;
                stdin.flush()?;
            }

            // Read response from daemon
            let response = {
                let mut stdout = daemon_stdout.lock().await;
                let mut resp = String::new();
                stdout.read_line(&mut resp)?;
                resp
            };

            // Send back to client
            ws_sender
                .send(tokio_tungstenite::tungstenite::Message::Text(response))
                .await?;
        } else if msg.is_close() {
            break;
        }
    }

    // Cleanup
    let _ = daemon.kill();
    Ok(())
}

fn spawn_daemon() -> Result<Child, std::io::Error> {
    Command::new("orthos-daemon")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
}
