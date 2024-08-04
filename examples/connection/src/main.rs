use potato::{NoiseStateMachine, Role};

use anyhow::{bail, Context};
use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use serde_json::Value;
use snow::{params::NoiseParams, Builder};
use std::io::Write;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::protocol::Message};

pub type Result<T=(), E=anyhow::Error> = std::result::Result<T, E>;
const IP_PORT: &str = "127.0.0.1:3030";

fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload (as a json): ");
    std::io::stdin()
        .read_line(&mut payload)
        .expect("Failed to read line");
    payload.trim().to_string()
}

async fn start_websocket_client(public_static_key: Vec<u8>) -> Result {
    let url = format!("ws://{}", IP_PORT);
    let (ws_stream, _response) = connect_async(&url).await.context("Failed to connect")?;
    let (mut write, mut read) = ws_stream.split();

    //receiver function
    let up_func = js_sys::Function::new_with_args("return function(a) { return 'Hello from the client UP'; }", 0);
    //sender function
    let down_func = js_sys::Function::new_with_args("return function(a) { return 'Hello from the client DOWN'; }", 0);

    let mut state_machine = NoiseStateMachine::new_init(Role::Initiator, up_func, down_func, public_static_key);

    let msg = payload_generator();

    let msg = state_machine.handle_connection(msg)?;

    write.send(Message::binary(&state_machine.get_init_msg())).await?;

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

                let msg = String::new();
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



#[tokio::main]
async fn main() {
    print!("Insert the public static key of the server: ");
    // Read a Vec<u8> from stdin
    let mut input = String::new();
    print!("Enter comma-separated byte values: ");
    std::io::stdout().flush().unwrap(); // Ensure the prompt is displayed

    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    // Trim the input and remove the surrounding brackets
    let trimmed = input.trim();
    let without_brackets = &trimmed[1..trimmed.len() - 1];

    // Convert the input string to Vec<u8>
    let public_static_key: Vec<u8> = without_brackets
        .split(',')
        .map(|x| x.trim().parse().expect("Invalid number"))
        .collect();
    match start_websocket_client(public_static_key).await {
        Err(e) => eprintln!("Error: {:?}", e),
        _ => (),
    }
}
