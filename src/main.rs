mod local_server;

use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use snow::{params::NoiseParams, Builder};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::protocol::Message};

pub type Error = Box<dyn std::error::Error>;

const IP_PORT: &str = "127.0.0.1:3030";
const PROXY_IP_PORT: &str = "127.0.0.1:9999";
lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap(); //Might have to change to XK
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
}

async fn start_websocket_server() {
    let listener = TcpListener::bind(IP_PORT).await.expect("Failed to bind");
    println!("WebSocket server running on {}", IP_PORT);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
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

async fn handle_connection(stream: tokio::net::TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws_stream) => ws_stream,
        Err(e) => {
            eprintln!("Failed to accept WebSocket connection: {:?}", e);
            return;
        }
    };
    let (mut write, mut read) = ws_stream.split();

    let builder = Builder::new(NOISE_PARAMS.clone());
    let static_key = builder.generate_keypair().unwrap().private;
    let mut noise = builder
        .local_private_key(&static_key)
        .unwrap()
        .psk(3, &SECRET.clone())
        .unwrap()
        .build_responder()
        .unwrap();
    let mut buf = vec![0u8; 65535];

    // <- e
    let msg = read.next().await.unwrap().unwrap();
    noise.read_message(&msg.into_data(), &mut buf).unwrap();

    // -> e, ee, s, es
    let len = noise.write_message(&[], &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    // <- s, se
    let msg = read.next().await.unwrap().unwrap();
    noise.read_message(&msg.into_data(), &mut buf).unwrap();

    let mut noise = noise.into_transport_mode().unwrap();
    let stop = false;

    while !stop {
        if let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            let len = noise.read_message(&msg.into_data(), &mut buf).unwrap();
            let msg = String::from_utf8_lossy(&buf[..len]);
            let (response, msg_id) = local_server::run_client(PROXY_IP_PORT, msg).await.unwrap();
            println!("MsgId: {:?}", msg_id);
            let response = create_json(response, &msg_id);
            let len = noise.write_message(response.as_bytes(), &mut buf).unwrap();
            write.send(Message::binary(&buf[..len])).await.unwrap();
            println!("Answer sent.");
        }
    }
    println!("Connection closed.");
}

async fn start_websocket_client() {
    let url = format!("ws://{}", IP_PORT);
    let (mut write, mut read) = match connect_async(&url).await {
        Ok((ws_stream, _)) => ws_stream.split(),
        Err(e) => {
            eprintln!("Failed to connect: {:?}", e);
            return;
        }
    };

    let builder = Builder::new(NOISE_PARAMS.clone());
    let static_key = builder.generate_keypair().unwrap().private;
    let mut noise = builder
        .local_private_key(&static_key)
        .unwrap()
        .psk(3, &SECRET.clone())
        .unwrap()
        .build_initiator()
        .unwrap();
    let mut buf = vec![0u8; 65535];

    // -> e
    let len = noise.write_message(&[], &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    // <- e, ee, s, es
    let msg = read.next().await.unwrap().unwrap();
    noise.read_message(&msg.into_data(), &mut buf).unwrap();

    // -> s, se
    let len = noise.write_message(&[], &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    let mut noise = noise.into_transport_mode().unwrap();
    println!("Session established...");

    let msg = payload_generator();
    let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
    write.send(Message::binary(&buf[..len])).await.unwrap();

    loop {
        if let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            let len = noise.read_message(&msg.into_data(), &mut buf).unwrap();
            if String::from_utf8_lossy(&buf[..len]).eq("exit") {
                break;
            }
            println!("Server said: {}", String::from_utf8_lossy(&buf[..len]));

            let msg = payload_generator();
            let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
            write.send(Message::binary(&buf[..len])).await.unwrap();
        }
    }
    println!("Connection closed.");
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
        server_mode = if 's' == input.trim().chars().next().unwrap() {
            0
        } else if 'p' == input.trim().chars().next().unwrap() {
            1
        } else if 'c' == input.trim().chars().next().unwrap() {
            2
        } else {
            4
        };
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
        start_websocket_client().await;
    } else {
        println!("Invalid mode");
    }
}
