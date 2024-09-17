mod local_server;

use anyhow::{bail, Context};
use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use serde_json::Value;
use snow::{params::NoiseParams, Builder};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::protocol::Message};
use std::io::Write;

pub type Result<T=(), E=anyhow::Error> = std::result::Result<T, E>;
//pub type Error = Box<dyn std::error::Error>;

const IP_PORT: &str = "127.0.0.1:3030";
const PROXY_IP_PORT: &str = "127.0.0.1:9999";
lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s".parse().expect("Parsing a constant will cause no error"); //Might have to change to XK
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
}

async fn start_websocket_server() {
    let listener = TcpListener::bind(IP_PORT).await.expect("Failed to bind");
    println!("WebSocket server running on {}", IP_PORT);

    let builder = Builder::new(NOISE_PARAMS.clone());
    let static_key = builder.generate_keypair().unwrap();
    println!("Public static key: {:?}", static_key.public.clone());

    while let Ok((stream, _)) = listener.accept().await {
        println!("New connection detected...");
        tokio::spawn(handle_connection(stream, static_key.private.clone()));
    }
}

fn create_json(response: String, msg_id: &str) -> String {
    let mut json = String::new();
    json.push_str("{");
    json.push_str("\"jsonrpc\": \"2.0\", ");
    json.push_str("\"result\": ");
    json.push_str(&response);
    json.push_str(", ");
    json.push_str("\"id\": ");
    json.push_str(msg_id);
    json.push_str("}");
    json
}

async fn handle_connection(stream: tokio::net::TcpStream, static_key: Vec<u8>) -> Result {
    let ws_stream = accept_async(stream).await.context("Failed to accept WebSocket connection")?;
    let (mut write, mut read) = ws_stream.split();

    let builder = Builder::new(NOISE_PARAMS.clone());
    let mut noise = builder
        .local_private_key(&static_key)?
        .psk(3, &SECRET.clone())?
        .build_responder()?;

    let mut buf = vec![0u8; 65535];

    // <- e
    let msg = match read.next().await {
        Some(Ok(msg)) => msg,
        Some(Err(e)) => bail!("Websocket error: {:?}", e),
        None => bail!("Handshake failed on receiving information"),
    };
    noise.read_message(&msg.into_data(), &mut buf)?;

    // -> e, ee, s, es
    let len = noise.write_message(&[], &mut buf)?;
    write.send(Message::binary(&buf[..len])).await?;

    // <- s, se
    let msg = match read.next().await {
        Some(Ok(msg)) => msg,
        Some(Err(e)) => bail!("Websocket error: {:?}", e),
        None => bail!("Handshake failed on receiving information"),
    };
    noise.read_message(&msg.into_data(), &mut buf)?;

    let mut noise = noise.into_transport_mode()?;
    println!("Session established...");

    loop {
        match read.next().await {
            Some(Ok(msg)) => {
                println!("Received a message...-{}-end of message", msg);
                let len = noise.read_message(&msg.into_data(), &mut buf)?;
                let msg = String::from_utf8_lossy(&buf[..len]);
                println!("Client said: {}", msg);
                let (response, msg_id) = local_server::run_client(PROXY_IP_PORT, msg).await?;

                let response = create_json(response, &msg_id);
                let len = noise.write_message(response.as_bytes(), &mut buf)?;
                println!("Sending a message...");
                write.send(Message::binary(&buf[..len])).await?;
                println!("Sent a message...");
            }
            Some(Err(e)) => {
                eprintln!("Websocket error: {:?}", e);
                break;
            }
            None => { break; }
        }
    }
    println!("Connection closed.");
    Ok(())
}

async fn start_websocket_client(public_static_key: Vec<u8>) -> Result {
    let url = format!("ws://{}", IP_PORT);
    let (ws_stream, _response) = connect_async(&url).await.context("Failed to connect")?;
    let (mut write, mut read) = ws_stream.split();

    let builder = Builder::new(NOISE_PARAMS.clone());
    let builder = builder.remote_public_key(&public_static_key)?;
    let static_key = builder.generate_keypair()?.private;
    let mut noise = builder
        .local_private_key(&static_key)?
        .psk(3, &SECRET.clone())?
        .build_initiator()?;
    let mut buf = vec![0u8; 65535];

    // -> e
    let len = noise.write_message(&[], &mut buf)?;
    write.send(Message::binary(&buf[..len])).await?;

    // <- e, ee, s, es
    let msg: Message = match read.next().await {
        Some(Ok(msg)) => msg,
        Some(Err(e)) => bail!("Websocket error: {:?}", e),
        None => bail!("Handshake failed on receiving information"),
    };
    noise.read_message(&msg.into_data(), &mut buf)?;

    // -> s, se
    let len = noise.write_message(&[], &mut buf)?;
    write.send(Message::binary(&buf[..len])).await?;

    let mut noise = noise.into_transport_mode()?;
    println!("Session established...");

    let msg = payload_generator();
    let len = noise.write_message(&(msg.as_bytes()), &mut buf)?;
    write.send(Message::binary(&buf[..len])).await?;

    loop {
        match read.next().await {
            Some(Ok(msg)) => {
                let len = noise.read_message(&msg.into_data(), &mut buf)?;
                let response = String::from_utf8_lossy(&buf[..len]);
                println!("Server said: {}", response);
                let msg_json: Value = serde_json::from_str(&response)?;
                let is_exit = match msg_json.get("result") {
                    Some(is_exit) => is_exit == "exit",
                    None => bail!("Invalid result: {:?}", response),
                };
                if is_exit {
                    break;
                }

                let msg = payload_generator();
                let len = noise.write_message(&(msg.as_bytes()), &mut buf)?;
                write.send(Message::binary(&buf[..len])).await?;
            }
            Some(Err(e)) => {
                eprintln!("Websocket error: {:?}", e);
                break;
            }
            None => { break; }
        }
    }
    println!("Connection closed.");
    Ok(())
}

fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload (as a json): ");
    std::io::stdin()
        .read_line(&mut payload)
        .expect("Failed to read line");
    payload.trim().to_string()
}

#[tokio::main]
async fn main() {
    let server_mode: u8;
    if std::env::args().len() > 1 {
        server_mode = std::env::args().next_back().map_or(0, |arg| {
            if arg == "-s" || arg == "--server" {
                0
            } else if arg == "-p" || arg == "--proxy" {
                1
            } else if arg == "-c" || arg == "--client" {
                2
            } else {
                4
            }
        });
    } else {
        println!("Mode? [s = server] [p = proxy] [c = client] [default = invalid]");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        server_mode = match input.trim().chars().next() {
            Some('s') => 0,
            Some('p') => 1,
            Some('c') => 2,
            _ => 4,
        }
    }
    if server_mode == 0 {
        println!("Server mode");
        start_websocket_server().await;
    } else if server_mode == 1 {
        println!("Proxy mode");
        let _ = local_server::run_server().await;
        println!("Proxy mo2de");
    } else if server_mode == 2 {
        println!("Client mode");
        print!("Insert the public static key of the server: ");
        // Read a Vec<u8> from stdin
        let mut input = String::new();
        print!("Enter comma-separated byte values (ex.: [1, 2, 3]): ");
        std::io::stdout().flush().unwrap();  // Ensure the prompt is displayed

        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        // Trim the input and remove the surrounding brackets
        let trimmed = input.trim();
        let without_brackets = &trimmed[1..trimmed.len()-1];

        // Convert the input string to Vec<u8>
        let public_static_key: Vec<u8> = without_brackets
            .split(',')
            .map(|x| x.trim().parse().expect("Invalid number"))
            .collect();
        println!("Public static key: {:?}", public_static_key);
        match start_websocket_client(public_static_key).await {
            Err(e) => eprintln!("Error: {:?}", e),
            _ => (),
        }
    } else {
        println!("Invalid mode");
    }
}