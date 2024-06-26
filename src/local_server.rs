use jsonrpsee::{
    core::{client::ClientT, RpcResult},
    proc_macros::rpc,
    server::{ServerBuilder},
    ws_client::WsClientBuilder,
};
use serde_json::Value;
use std::{borrow::Cow};
use anyhow::bail;

//pub type Error = Box<dyn std::error::Error>;
pub type Result<T=(), E=anyhow::Error> = std::result::Result<T, E>;

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

pub async fn run_server() -> Result {
    let server = ServerBuilder::default()
        .ws_only()
        .build(PROXY_IP_PORT)
        .await?;

    let handle = server.start(RpcImpl.into_rpc());

    handle.stopped().await;

    Ok(())
}

pub async fn run_client(ip_port: &str, msg: Cow<'_, str>) -> Result<(String, String), anyhow::Error> {
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
    let method = match msg_json.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => bail!("Invalid method"),
    };
    let id = match msg_json.get("id") {
        Some(id) => id.to_string(),
        None => bail!("Invalid id"),
    };
    let answer = match method {
        "add" => {
            let params = match msg_json.get("params") {
                Some(params) => match params.as_array() {
                    Some(params) => params,
                    None => bail!("Invalid parameters"),
                },
                None => bail!("Invalid parameters"),
            };
            let a = match params[0].as_i64() {
                Some(a) => a,
                None => bail!("Invalid first parameter"),
            };
            let b = match params[1].as_i64() {
                Some(b) => b,
                None => bail!("Invalid second parameter"),
            };
            let response: Value = client.request("add", jsonrpsee::rpc_params![a, b]).await?;
            println!("add response (for {}+{}): {}", a, b, response);
            Ok::<(String, String), anyhow::Error>((response.to_string(), id))
        }
        "exit" => {
            let response: Value = client.request("exit", jsonrpsee::rpc_params![]).await?;
            println!("exit response: {}", response);
            Ok((response.to_string(), id))
        }
        "say_hello" => {
            let response: Value = client
                .request("say_hello", jsonrpsee::rpc_params![])
                .await?;
            println!("say_hello response: {}", response);
            Ok((response.to_string(), id))
        }
        _ => Ok(("Invalid method".to_string(), id)),
    }?;

    Ok(answer)
}
