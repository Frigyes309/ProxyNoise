#![allow(warnings)]

mod lower_part;
mod lib;

use std::cmp::PartialEq;
use snow::{HandshakeState, params::NoiseParams, Builder};
use crate::NoiseHandshake::*;
use crate::Role::*;
use tokio_tungstenite::tungstenite::protocol::Message;
use lazy_static::lazy_static;
use tokio::net::TcpStream;
use wasm_bindgen::prelude::*;

pub type ResultSender = Result<(NoiseHandshake, &'static mut Vec<u8>, usize), anyhow::Error>;
pub type ResultResponder = Result<(NoiseHandshake, &'static mut Vec<u8>), anyhow::Error>;

lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().expect("Parsing a constant will cause no error");
    static ref SECRET: [u8; 32] = *b"Random 32 characters long secret";
    // static ref STATIC_KEY: [u8; 32] = *b"Random 32 characters long static key";

}

enum NoiseHandshake {
    WaitingForConnection,
    ReceivedPublicStaticKey,
    SentEphemeralPublicKey,
    ReceivedEphemeralPublicKey,
    SentStaticPublicKey,
    ReadyToSendData,
    WaitingForRequest,
    WaitingForResponse,
}

#[derive(PartialEq)]
enum Role {
    Initiator,
    Responder,
}

struct NoiseStateMachine<'a> {
    state: NoiseHandshake,
    noise: HandshakeState,
    role: Role,
    server_connection: TcpStream,
    callback_fn: &'a js_sys::Function,
}

#[cfg(feature = "client")]
trait NoiseClient {
    fn initiator_start_handshake(noise: &mut HandshakeState, buf: &'static mut Vec<u8>) -> ResultSender;
    fn initiator_second_phase(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>) -> ResultResponder;
    fn initiator_third_phase(noise: &mut HandshakeState, buf: &'static mut Vec<u8>) -> ResultSender;

    fn initiator_handle_connection(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>, state: NoiseHandshake) -> ResultSender;

    fn initiator_send_request() -> ResultSender;

    fn initiator_handle_response() -> ResultSender;
}

#[cfg(feature = "client")]
impl NoiseClient for NoiseStateMachine {
    fn initiator_start_handshake(noise: &mut HandshakeState, buf: &'static mut Vec<u8>) -> ResultSender {
        let len = noise.write_message(&[], buf)?;
        Ok((SentEphemeralPublicKey, buf, len))
    }

    fn initiator_second_phase(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>) -> ResultResponder {
        noise.read_message(&msg.into_data(), buf)?;
        Ok((ReceivedEphemeralPublicKey, buf))
    }

    fn initiator_third_phase(noise: &mut HandshakeState, buf: &'static mut Vec<u8>) -> ResultSender {
        let len = noise.write_message(&[], buf)?;
        Ok((ReadyToSendData, buf, len))
    }

    fn initiator_handle_connection(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>, state: NoiseHandshake) -> ResultSender {
        match state {
            WaitingForConnection => Self::initiator_start_handshake(noise, buf),
            SentEphemeralPublicKey => match Self::initiator_second_phase(noise, msg, buf) {
                Ok((state, buf)) => Ok((state, buf, 0)),
                Err(e) => Err(e),
            }
            ReceivedEphemeralPublicKey => Self::initiator_third_phase(noise, buf),
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


impl NoiseStateMachine {
    fn new(role: Role, addr_port: &str, func: &js_sys::Function) -> Self {
        let builder = Builder::new(NOISE_PARAMS.clone());
        let static_key = builder.generate_keypair().unwrap().private;
        let noise = match builder
            .local_private_key(&static_key)?
            .psk(3, &SECRET.clone())?
            .build_initiator()? {
                Ok(noise) => noise,
                Err(e) => todo!("Error: {:?}", e)
        };
        let connection = match TcpStream::connect(addr_port)? {
            Ok(connection) => connection,
            Err(e) => todo!("Error: {:?}", e)
        };
        Self {
            state: WaitingForConnection,
            noise,
            role,
            server_connection: connection,
            callback_fn: func,
        }
    }

    fn handle_connection(self: &mut self, msg: Option<Message>, mut buf: Vec<u8>) -> () {
        let msg = match msg {
            Some(msg) => msg,
            None => "".into()
        };
        if self.role == Initiator {
            #[cfg(feature = "client")]
            match self.state {
                WaitingForConnection => {
                    let encoded = Self::initiator_start_handshake(&mut self.noise, &mut buf);
                    match encoded {
                        Ok((state, buf, len)) => {
                            self.connection.write_all(&buf[..len]);
                            self.state = state;
                        },
                        Err(e) => todo!("Error: {:?}", e)
                    }
                }
                SentEphemeralPublicKey => {
                    let decoded = Self::initiator_second_phase(&mut self.noise, msg, buf);
                    match decoded {
                        Ok((state, buf)) => {
                            self.state = state;
                            self.handle_connection(self, None, buf);
                        },
                        Err(e) => todo!("Error: {:?}", e)
                    }
                },
                ReceivedEphemeralPublicKey => {
                    let encoded = Self::responder_second_phase(&mut self.noise, buf);
                    match encoded {
                        Ok((state, buf, len)) => {
                            self.connection.write_all(&buf[..len]);
                            self.state = state;
                        }
                        Err(e) => todo!("Error {:?}", e)
                    }
                },

                ReadyToSendData => {
                    Self::initiator_send_request()
                },
                WaitingForRequest => Self::server_handle_request(),
                WaitingForResponse => Self::server_send_response(),
            }
        } else {
            #[cfg(feature = "server")]
            match self.state {
                WaitingForConnection => {
                    let decode = Self::responder_start_handshake(&mut self.noise, msg, buf);
                    match decode {
                        Ok((state, buf)) => {
                            self.state = state;
                        },
                        Err(e) => todo!("Error: {:?}", e)
                    }
                },
                ReceivedPublicStaticKey => Self::responder_start_handshake(&mut self.noise, msg, buf),
                SentEphemeralPublicKey => Self::initiator_second_phase(&mut self.noise, msg, buf),
                ReceivedEphemeralPublicKey => Self::responder_second_phase(&mut self.noise, buf),
                SentStaticPublicKey => Self::initiator_third_phase(&mut self.noise, buf),
                ReadyToSendData => Self::initiator_send_request(),
                WaitingForRequest => Self::server_handle_request(),
                WaitingForResponse => Self::server_send_response(),
            }
        }
    }
}

#[cfg(feature = "server")]
trait NoiseServer {
    fn responder_start_handshake(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>) -> ResultResponder;
    fn responder_second_phase(noise: &mut HandshakeState, buf: &'static mut Vec<u8>) -> ResultSender;
    fn responder_third_phase(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>) -> ResultResponder;

    fn server_handle_connection(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>, state: NoiseHandshake) -> ResultSender;

    fn server_handle_request() -> ResultSender;

    fn server_send_response() -> ResultSender;
}

#[cfg(feature = "server")]
impl NoiseServer for NoiseStateMachine {
    fn responder_start_handshake(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>) -> ResultResponder {
        noise.read_message(&msg.into_data(), buf)?;
        Ok((ReceivedEphemeralPublicKey, buf))
    }

    fn responder_second_phase(noise: &mut HandshakeState, buf: &'static mut Vec<u8>) -> ResultSender {
        let len = noise.write_message(&[], buf)?;
        Ok((SentStaticPublicKey, buf, len))
    }

    fn responder_third_phase(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>) -> ResultResponder {
        noise.read_message(&msg.into_data(), buf)?;
        Ok((WaitingForRequest, buf))
    }

    fn server_handle_connection(noise: &mut HandshakeState, msg: Message, buf: &'static mut Vec<u8>, state: NoiseHandshake) -> ResultSender {
        match state {
            WaitingForConnection => match Self::responder_start_handshake(noise, msg, buf) {
                Ok((state, buf)) => Ok((state, buf, 0)),
                Err(e) => Err(e),
            },
            ReceivedEphemeralPublicKey => Self::responder_second_phase(noise, buf),
            SentStaticPublicKey => match Self::responder_third_phase(noise, msg, buf) {
                Ok((state, buf)) => Ok((state, buf, 0)),
                Err(e) => Err(e),
            },
            WaitingForRequest => Self::server_handle_request(),
            WaitingForResponse => Self::server_send_response(),
            _ => Err(anyhow::anyhow!("Invalid state")),
        }
    }

    fn server_handle_request() -> ResultSender {
        Err(anyhow::anyhow!("Not implemented"))
    }

    fn server_send_response() -> ResultSender {
        Err(anyhow::anyhow!("Not implemented"))
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