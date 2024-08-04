#![allow(warnings)]
#![allow(unused)]

mod tomato;

use anyhow::bail;
use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use serde_json::Value;
use snow::{params::NoiseParams, Builder, HandshakeState, TransportState};
use std::cmp::PartialEq;
use std::hash::Hasher;
use std::io::Write;
use std::string::String;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use wasm_bindgen::prelude::*;

// pub type ResultSender = Result<(NoiseHandshake, &'static mut Vec<u8>, usize), anyhow::Error>;
// pub type ResultResponder = Result<(NoiseHandshake, &'static mut Vec<u8>), anyhow::Error>;

lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().expect("Parsing a constant will cause no error");
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
    // static ref STATIC_KEY: [u8; 32] = *b"Random 32 characters long static key";

}

// enum NoiseHandshake {
//     WaitingForConnection,
//     ReadyToSendData,
//     WaitingForRequest,
//     WaitingForResponse,
// }

// #[derive(PartialEq)]
// enum Role {
//     Initiator,
//     Responder,
// }

struct NoiseStateMachine<'a> {
    noise: TransportState,
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    callback_fn: &'a js_sys::Function,
    buf: Vec<u8>,
}

/*trait NoiseClient {

    fn initiator_handle_connection(noise: &mut HandshakeState, msg: Message, state: NoiseHandshake) -> ResultSender;

    fn initiator_send_request() -> ResultSender;

    fn initiator_handle_response() -> ResultSender;
}

impl NoiseClient for NoiseStateMachine {

    fn initiator_handle_connection(&mut self) -> ResultSender {
        match self.state {
            WaitingForConnection => {
                let len = noise.write_message(&[], buf).unwrap();
                server_connection.write_all(&buf[..len]);
                connection..send(Message::binary(&buf[..len]));

                // <- e, ee, s, es
                let msg = read.next().await.unwrap().unwrap();
                noise.read_message(&msg.into_data(), &mut buf).unwrap();

                // -> s, se
                let len = noise.write_message(&[], &mut buf).unwrap();
                write.send(Message::binary(&buf[..len])).await.unwrap();
            }
            SentEphemeralPublicKey => match Self::initiator_second_phase(noise, msg, buf) {
                Ok((state, buf)) => Ok((state, buf, 0)),
                Err(e) => Err(e),
            }
            ReadyToSendData => Self::initiator_send_request(),
            WaitingForResponse => Self::initiator_handle_response(),
            _ => Err(anyhow::anyhow!("Invalid state")),
        }
    }

    fn initiator_send_request() -> ResultSender {
        Err(anyhow::anyhow!("Not implemented"))
    }

    fn initiator_handle_response() -> ResultSender {
        Err(anyhow::anyhow!("Not implemented"))
    }
}
*/

impl NoiseStateMachine<'_> {
    fn new(addr_port: &str, func: &js_sys::Function) -> anyhow::Result<Self> {
        let builder = Builder::new(NOISE_PARAMS.clone());
        let static_key = builder.generate_keypair().unwrap().private;
        let mut noise: HandshakeState = builder
            .local_private_key(&static_key)?
            .psk(3, &SECRET.clone())?
            .build_initiator()?;
        let mut buf = vec![0u8; 65535];
        // let ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>> = ;
        // let (mut write, mut read) = ws_stream.split();

        let len = noise.write_message(&[], &mut buf)?;
        write.send(Message::binary(&buf[..len]))?;

        // <- e, ee, s, es
        let msg: Message = match read.next()? {
            //TODO check if this is correct
            Some(Ok(msg)) => msg,
            Some(Err(e)) => panic!("Websocket error: {:?}", e),
            None => panic!("Handshake failed on receiving information"),
        };
        noise.read_message(&msg.into_data(), &mut buf)?;

        // -> s, se
        let len = noise.write_message(&[], &mut buf)?;
        write.send(Message::binary(&buf[..len]))?;

        let mut noise = noise.into_transport_mode()?;

        Ok(Self {
            noise,
            write,
            read,
            callback_fn: func,
            buf,
        })
    }

    fn handle_connection(
        self: &mut Self,
        msg: String,
        mut buf: Vec<u8>,
        request: bool,
    ) -> Result<(), anyhow::Error> {
        if request {
            let msg = if !msg.is_empty() {
                Message::text(msg.as_str())
            } else {
                //TODO todo!("Empty message");
                Err(anyhow::anyhow!("Empty message"));
                panic!("Empty message");
            };
            let len = self.noise.write_message(&(msg.as_bytes()), &mut buf)?;
            self.write.send(Message::binary(&buf[..len]))?;
            Ok(())
        } else {
            let msg: Message = match self.read.next()? {
                //TODO check if this is correct
                Some(Ok(msg)) => msg,
                Some(Err(e)) => bail!("Websocket error: {:?}", e),
                None => bail!("Handshake failed on receiving information"),
            };
            let len = self.noise.read_message(&msg.into_data(), &mut buf)?;
            let response = String::from_utf8_lossy(&buf[..len]);
            let msg_js: JsValue = JsValue::from_str(&response);

            self.callback_fn.call1(&JsValue::NULL, &msg_js)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {
        // let mut initiator = crate::NoiseHandshake::ReceivedPublicStaticKey;
        // assert_eq!(result, 4);
    }
}
