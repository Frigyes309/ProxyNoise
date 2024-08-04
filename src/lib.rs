
use lazy_static::lazy_static;
use snow::{params::NoiseParams, HandshakeState};
use std::cmp::PartialEq;

#[cfg(any(feature = "server", feature = "client"))]
use wasm_bindgen::prelude::*;
#[cfg(any(feature = "server", feature = "client"))]
use snow::Builder;
#[cfg(any(feature = "server", feature = "client"))]
use crate::NoiseHandshake::{SentEphemeralPublicKey, WaitingForConnection};
#[cfg(any(feature = "server", feature = "client"))]
use crate::Role::Initiator;
#[cfg(any(feature = "server", feature = "client"))]
use anyhow::Error;
#[cfg(any(feature = "server", feature = "client"))]
use NoiseHandshake::*;
#[cfg(any(feature = "server", feature = "client"))]
use tokio_tungstenite::tungstenite::protocol::Message;
#[cfg(any(feature = "server", feature = "client"))]
type ResultHandler = Result<(NoiseHandshake, JsValue), anyhow::Error>;

lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .expect("Parsing a constant will cause no error");
}

#[allow(dead_code)]
enum NoiseHandshake {
    WaitingForConnection,
    ReceivedPublicStaticKey,
    SentEphemeralPublicKey,
    ReceivedEphemeralPublicKey,
    SentStaticPublicKey,
    HandshakeCompleted,
}

#[allow(dead_code)]
#[derive(PartialEq, Clone, Copy)]
enum Role {
    Initiator,
    Responder,
}

#[allow(dead_code)]
struct NoiseStateMachine<'a> {
    role: Role,
    state: NoiseHandshake,
    handshaker: Handshaker,
    up_func: &'a js_sys::Function,
    down_func: &'a js_sys::Function,
}

#[cfg(feature = "client")]
trait NoiseClient {
    fn initiator_start_handshake(&mut self) -> ResultHandler;
    fn initiator_second_phase(&mut self, msg: Message) -> ResultHandler;
    fn initiator_third_phase(&mut self) -> ResultHandler;

    fn initiator_send_request(&mut self) -> ResultHandler;

    fn initiator_handle_response(&mut self, msg: Message) -> ResultHandler;
}

struct Handshaker {
    noise: HandshakeState,
    buf: Vec<u8>,
}

#[cfg(feature = "client")]
impl NoiseClient for Handshaker {
    fn initiator_start_handshake(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((SentEphemeralPublicKey, msg))
    }

    fn initiator_second_phase(&mut self, msg: Message) -> ResultHandler {
        self.noise.read_message(&msg.into_data(), &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value("") {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((ReceivedEphemeralPublicKey, msg))
    }

    fn initiator_third_phase(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((HandshakeCompleted, msg))
    }

    fn initiator_send_request(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((HandshakeCompleted, msg))
    }

    fn initiator_handle_response(&mut self, msg: Message) -> ResultHandler {
        let start_len = self.buf.len();
        let len = self.noise.read_message(&msg.into_data(), &mut self.buf)?;
        let msg = &self.buf[..len];
        print!("Startlen: {:?}\nLen: {:?}", start_len, len);
        let msg = match serde_wasm_bindgen::to_value(msg) {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((ReceivedEphemeralPublicKey, msg))
    }
}

// #[cfg(feature = "server")]
/*trait NoiseServer {
    fn responder_start_handshake(
        noise: &mut HandshakeState,
        msg: Message,
        buf: &mut Vec<u8>,
    ) -> ResultResponder;
    fn responder_second_phase(noise: &mut HandshakeState, buf: &mut Vec<u8>) -> ResultSender;
    fn responder_third_phase(
        noise: &mut HandshakeState,
        msg: Message,
        buf: &mut Vec<u8>,
    ) -> ResultResponder;

    fn server_handle_connection(
        noise: &mut HandshakeState,
        msg: Message,
        buf: &mut Vec<u8>,
        state: NoiseHandshake,
    ) -> ResultSender;

    fn server_handle_request() -> ResultSender;

    fn server_send_response() -> ResultSender;
}

#[cfg(feature = "server")]
impl NoiseServer for HandshakeState {
    fn responder_start_handshake(
        noise: &mut HandshakeState,
        msg: Message,
        buf: &'static mut Vec<u8>,
    ) -> ResultResponder {
        noise.read_message(&msg.into_data(), buf)?;
        Ok((ReceivedEphemeralPublicKey, buf))
    }

    fn responder_second_phase(
        noise: &mut HandshakeState,
        buf: &'static mut Vec<u8>,
    ) -> ResultSender {
        let len = noise.write_message(&[], buf)?;
        Ok((SentStaticPublicKey, buf, len))
    }

    fn responder_third_phase(
        noise: &mut HandshakeState,
        msg: Message,
        buf: &'static mut Vec<u8>,
    ) -> ResultResponder {
        noise.read_message(&msg.into_data(), buf)?;
        Ok((WaitingForRequest, buf))
    }

    fn server_handle_connection(
        noise: &mut HandshakeState,
        msg: Message,
        buf: &'static mut Vec<u8>,
        state: NoiseHandshake,
    ) -> ResultSender {
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
}*/

#[cfg(any(feature = "server", feature = "client"))]
impl NoiseStateMachine<'_> {
    #[allow(dead_code)] // Called from outside
    fn new_init<'a>(
        role: Role,
        callback_up: &'a js_sys::Function,
        callback_down: &'a js_sys::Function,
        server_static_key: Vec<u8>,
    ) -> anyhow::Result<NoiseStateMachine<'a>, Error> {
        let builder = Builder::new(NOISE_PARAMS.clone());
        let builder = builder.remote_public_key(&server_static_key)?;
        let static_key = builder.generate_keypair()?.private;
        let secret: [u8; 32] = *b"Random 32 characters long secret";
        let noise: HandshakeState = builder
            .local_private_key(&static_key)?
            .psk(3, &secret.clone())?
            .build_initiator()?;
        let mut buf: Vec<u8> = vec![0u8; 65535];
        let mut handshaker = Handshaker {
            noise,
            buf: buf.clone(),
        };
        Ok(NoiseStateMachine {
            role,
            state: if Initiator == role {
                // -> e, es
                let len = handshaker.noise.write_message(&[], &mut buf)?;
                let msg = match serde_wasm_bindgen::to_value(&handshaker.buf[..len]) {
                    Ok(msg) => msg,
                    Err(e) => todo!("Error: {:?}", e),
                };
                let _ = callback_down.call1(&JsValue::NULL, &msg);
                SentEphemeralPublicKey
            } else {
                WaitingForConnection
            },
            handshaker,
            up_func: callback_up,
            down_func: callback_down,
        })
    }

    // send mode is true if the message is to be sent to the server from the client
    #[allow(dead_code)] // Called from outside
    fn handle_connection(&mut self, msg: Option<Message>, send_mode: bool) -> () {
        match self.role {
            Role::Initiator =>
            {
                #[cfg(feature = "client")]
                match self.state {
                    WaitingForConnection => {
                        let (new_state, msg) = match self.handshaker.initiator_start_handshake() {
                            Ok((new_state, msg)) => (new_state, msg),
                            Err(e) => todo!("Error: {:?}", e),
                        };
                        let _ = self.down_func.call1(&JsValue::NULL, &msg);
                        self.state = new_state;
                    }
                    SentEphemeralPublicKey => {
                        let msg = match msg {
                            Some(msg) => msg,
                            None => todo!("No message received"),
                        };
                        let (new_state, _msg) = match self.handshaker.initiator_second_phase(msg) {
                            Ok((new_state, msg)) => (new_state, msg),
                            Err(e) => todo!("Error: {:?}", e),
                        };
                        self.state = new_state;
                        self.handle_connection(None, true);
                    }
                    ReceivedEphemeralPublicKey => {
                        let (new_state, msg) = match self.handshaker.initiator_third_phase() {
                            Ok((new_state, msg)) => (new_state, msg),
                            Err(e) => todo!("Error: {:?}", e),
                        };
                        let _ = self.down_func.call1(&JsValue::NULL, &msg);
                        self.state = new_state;
                    }

                    HandshakeCompleted => {
                        if send_mode {
                            let (new_state, msg) = match self.handshaker.initiator_send_request() {
                                Ok((new_state, msg)) => (new_state, msg),
                                Err(e) => todo!("Error: {:?}", e),
                            };
                            let _ = self.down_func.call1(&JsValue::NULL, &msg);
                            self.state = new_state;
                        } else {
                            let msg = match msg {
                                Some(msg) => msg,
                                None => todo!("No message received"),
                            };
                            let (new_state, msg) =
                                match self.handshaker.initiator_handle_response(msg) {
                                    Ok((new_state, msg)) => (new_state, msg),
                                    Err(e) => todo!("Error: {:?}", e),
                                };
                            let _ = self.up_func.call1(&JsValue::NULL, &msg);
                            self.state = new_state;
                        }
                    }
                    _ => todo!("Invalid state"),
                }
            }
            Role::Responder => {
                // #[cfg(feature = "server")]
                /*match self.state {
                    WaitingForConnection => {
                        let decode = Self::responder_start_handshake(&mut self.noise, msg, buf);
                        match decode {
                            Ok((state, buf)) => {
                                self.state = state;
                            }
                            Err(e) => todo!("Error: {:?}", e),
                        }
                    }
                    ReceivedPublicStaticKey => {
                        Self::responder_start_handshake(&mut self.noise, msg, buf)
                    }
                    SentEphemeralPublicKey => {
                        Self::initiator_second_phase(&mut self.noise, msg, buf)
                    }
                    ReceivedEphemeralPublicKey => {
                        Self::responder_second_phase(&mut self.noise, buf)
                    }
                    SentStaticPublicKey => Self::initiator_third_phase(&mut self.noise, buf),
                    ReadyToSendData => Self::initiator_send_request(),
                    WaitingForRequest => Self::server_handle_request(),
                    WaitingForResponse => Self::server_send_response(),
                }*/
            }
        }
    }
}