use futures_util::{SinkExt, StreamExt};
#[allow(unused_imports)]
use lazy_static::lazy_static;
use snow::{Builder, params::NoiseParams};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{accept_async, connect_async};
use std::borrow::Cow;

const IP_PORT: &str = "127.0.0.1:9999";
lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
}

async fn start_websocket_server() {
    let listener = TcpListener::bind(IP_PORT).await.expect("Failed to bind");
    println!("WebSocket server running on {}", IP_PORT);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }
}

fn handle_client_raw_message(msg: Cow<str>, buf: Vec<u8>, mut stop: bool) -> (Vec<u8>, bool) {
    /*let mut str_real_len = String::new();
    let mut chars_iter = msg.chars().peekable();
    let mut start_len = 0;
    while let Some(c) = chars_iter.next() {
        if c == 's' {
            break;
        }
        start_len += 1;
        str_real_len.push(c);
    }
    start_len += 1;
    let real_len = str_real_len.parse::<usize>().unwrap() + start_len;
    let mut answer: Vec<u8> = Vec::new();
    if String::from_utf8_lossy(&buf[start_len..real_len]).eq("exit") {
        for c in "exit".bytes() {
            answer.push(c);
        }
        stop = true;
    } else {
        for i in String::from_utf8_lossy(&buf[start_len..real_len]).chars() {
            let mut c = i as char;
            match c {
                '1' => { c = '0'; }
                '0' => { c = '1'; }
                _ => {
                    if c.is_uppercase() {
                        c = c.to_lowercase().next().unwrap();
                    } else {
                        c = c.to_uppercase().next().unwrap();
                    }
                }
            }
            answer.push(c as u8);
        }
    }
    (answer, stop)*/
    let mut answer: Vec<u8> = Vec::new();
    let mut stop = false;
    if msg.eq("exit") {
        for c in "exit".bytes() {
            answer.push(c);
        }
        stop = true;
    } else {
        for i in msg.chars() {
            let mut c = i;
            match c {
                '1' => { c = '0'; }
                '0' => { c = '1'; }
                _ => {
                    if c.is_uppercase() {
                        c = c.to_lowercase().next().unwrap();
                    } else {
                        c = c.to_uppercase().next().unwrap();
                    }
                }
            }
            answer.push(c as u8);
        }
    }
    (answer, stop)
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
    let mut stop = false;

    while !stop {
        if let Some(msg) = read.next().await {
            let msg = msg.unwrap();
            let len = noise.read_message(&msg.into_data(), &mut buf).unwrap();
            let msg = String::from_utf8_lossy(&buf[..len]);
            println!("Client said: {}", msg);
            let (answer, stop2) = handle_client_raw_message(msg, buf.clone(), stop);
            stop = stop2;
            let len = noise.write_message(&answer, &mut buf).unwrap();
            write.send(Message::binary(&buf[..len])).await.unwrap();
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
    println!("Message sent.");

    loop {
        let msg = read.next().await.unwrap().unwrap();
        let len = noise.read_message(&msg.into_data(), &mut buf).unwrap();
        if String::from_utf8_lossy(&buf[..len]).eq("exit") {
            break;
        }
        println!("Server said: {}", String::from_utf8_lossy(&buf[..len]));


        let msg = payload_generator();
        let len = noise.write_message(&(msg.as_bytes()), &mut buf).unwrap();
        write.send(Message::binary(&buf[..len])).await.unwrap();
        println!("Message sent.");
    }
    println!("Connection closed.");
}

fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload: ");
    std::io::stdin().read_line(&mut payload).expect("Failed to read line");
    //let payload = format!("{}s{}", payload.trim().len(), payload);
    payload.trim().to_string()
}

#[tokio::main]
async fn main() {
    //If start has been called with s as argument, the server mode will be started
    //If start has been called with c as argument, the client mode will be started
    #[allow(unused_assignments)]
        let mut server_mode: bool = false;
    if std::env::args().len() > 1 {
        server_mode = std::env::args().next_back().map_or(true, |arg| arg == "-s" || arg == "--server")
    } else {
        println!("Mode? [s = server]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        server_mode = 's' == input.trim().chars().next().unwrap();
    }
    if server_mode {
        println!("Server mode");
        start_websocket_server().await;
    } else {
        println!("Client mode");
        start_websocket_client().await;
    }
}
