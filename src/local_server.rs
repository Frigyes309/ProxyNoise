use std::{borrow::Cow, sync::Arc};
use futures_util::{SinkExt, StreamExt};
use jsonrpsee::{
    core::{client::ClientT, RpcResult},
    proc_macros::rpc,
    server::{Server, ServerBuilder},
    types::{ErrorCode, Params},
    ws_client::WsClientBuilder,
    RpcModule,
};
use lazy_static::lazy_static;
use serde_json::Value;
use tokio::{
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

pub type Error = Box<dyn std::error::Error>;

const PROXY_IP_PORT: &str = "127.0.0.1:9999";

#[rpc(server, client)]
pub trait Rpc {
    #[method(name = "add")]
    fn add(&self, a: i64, b: i64) -> RpcResult<i64>;

    #[method(name = "exit")]
    fn exit(&self) -> RpcResult<String>;

    #[method(name = "say_hello")]
    fn say_hello(&self) -> RpcResult<String>;
}

struct RpcImpl;

impl RpcServer for RpcImpl {
    fn add(&self, a: i64, b: i64) -> RpcResult<i64> {
        Ok(a + b)
    }

    fn exit(&self) -> RpcResult<String> {
        Ok(String::from("exit"))
    }

    fn say_hello(&self) -> RpcResult<String> {
        Ok(String::from("Hello, World!"))
    }
}

fn payload_generator() -> String {
    let mut payload = String::new();
    println!("Enter the payload: ");
    std::io::stdin()
        .read_line(&mut payload)
        .expect("Failed to read line");
    payload.trim().to_string()
}

pub async fn run_server() -> Result<(), Error> {
    let server = ServerBuilder::default()
        .ws_only()
        .build(PROXY_IP_PORT)
        .await?;

    let handle = server.start(RpcImpl.into_rpc());

    handle.stopped().await;

    Ok(())
}

pub async fn run_client(ip_port: &str, msg: Cow<'_, str>) -> Result<(String, String), Error> {
    let url = format!("ws://{}", ip_port);
    let client = WsClientBuilder::new().build(&url).await?;

    println!(
        "Connection {}",
        if client.is_connected() {
            "was successful"
        } else {
            "failed"
        }
    );

    let msg_json: Value = serde_json::from_str(&msg)?;
    let method = msg_json.get("method")
        .and_then(Value::as_str)
        .ok_or("error")?;
    let id = msg_json.get("id").ok_or("error")?.to_string();
    let answer = match method {
        "add" => {
            let params = msg_json.get("params").unwrap().as_array().unwrap();
            let a = params[0].as_i64().unwrap();
            let b = params[1].as_i64().unwrap();
            let response: Value = client.request("add", jsonrpsee::rpc_params![a, b]).await?;
            println!("add response (for {}+{}): {}", a, b, response);
            Ok::<(String, String), Error>((response.to_string(), id))
        }
        "exit" => {
            let response: Value = client.request("exit", jsonrpsee::rpc_params![]).await?;
            println!("exit response: {}", response);
            Ok((response.to_string(), id))
        }
        "say_hello" => {
            let response: Value = client.request("say_hello", jsonrpsee::rpc_params![]).await?;
            println!("say_hello response: {}", response);
            Ok((response.to_string(), id))
        }
        _ => Ok(("Invalid method".to_string(), id)),
    }?;

    Ok(answer)
}

