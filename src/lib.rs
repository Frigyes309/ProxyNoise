use lazy_static::lazy_static;
use snow::{params::NoiseParams, HandshakeState};
use wasm_bindgen::prelude::*;
use snow::Builder;
use crate::NoiseHandshake::{SentEphemeralPublicKey, WaitingForConnection};
use NoiseHandshake::*;
use std::fmt;
type ResultHandler = Result<(NoiseHandshake, JsValue), anyhow::Error>;

lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .expect("Parsing a constant will cause no error");
}

enum NoiseHandshake {
    WaitingForConnection,
    ReceivedPublicStaticKey,
    SentEphemeralPublicKey,
    ReceivedEphemeralPublicKey,
    SentStaticPublicKey,
    HandshakeCompleted,
}

#[wasm_bindgen(js_name = NoiseStateMachine)]
struct NoiseStateMachine {
    role: bool, //true for client, false for server
    state: NoiseHandshake,
    handshaker: Handshaker,
    up_func: js_sys::Function,
    down_func: js_sys::Function,
}

trait NoiseClient {
    fn initiator_start_handshake(&mut self) -> ResultHandler;
    fn initiator_second_phase(&mut self, msg: String) -> ResultHandler;
    fn initiator_third_phase(&mut self) -> ResultHandler;
    fn initiator_send_request(&mut self) -> ResultHandler;
    fn initiator_handle_response(&mut self, msg: String) -> ResultHandler;
}

struct Handshaker {
    noise: HandshakeState,
    buf: Vec<u8>,
}

impl NoiseClient for Handshaker {
    fn initiator_start_handshake(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((SentEphemeralPublicKey, msg))
    }

    fn initiator_second_phase(&mut self, msg: String) -> ResultHandler {
        self.noise.read_message(&msg.as_bytes(), &mut self.buf)?;
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

    fn initiator_handle_response(&mut self, msg: String) -> ResultHandler {
        // let start_len = self.buf.len();
        let len = self.noise.read_message(&msg.as_bytes(), &mut self.buf)?;
        let msg = &self.buf[..len];
        // print!("Startlen: {:?}\nLen: {:?}", start_len, len);
        let msg = match serde_wasm_bindgen::to_value(msg) {
            Ok(msg) => msg,
            Err(e) => todo!("Error: {:?}", e),
        };
        Ok((ReceivedEphemeralPublicKey, msg))
    }
}

#[wasm_bindgen(js_class = NoiseStateMachine)]
impl NoiseStateMachine {

    #[wasm_bindgen(constructor)]
    pub fn new_init(
        role: bool,
        callback_up: js_sys::Function,
        callback_down: js_sys::Function,
        server_static_key: Vec<u8>,
    ) -> Result<NoiseStateMachine, JsValue> {
        let builder = Builder::new(NOISE_PARAMS.clone());
        let builder = builder.remote_public_key(&server_static_key).map_err(NoiseError::from)?;
        let static_key = builder.generate_keypair().map_err(NoiseError::from)?.private;
        let secret: [u8; 32] = *b"Random 32 characters long secret";
        let noise: HandshakeState = builder
            .local_private_key(&static_key).map_err(NoiseError::from)?
            .psk(3, &secret).map_err(NoiseError::from)?
            .build_initiator().map_err(NoiseError::from)?;
        let mut buf: Vec<u8> = vec![0u8; 65535];
        let mut handshaker = Handshaker {
            noise,
            buf: buf.clone(),
        };

        let state = if role {
            let len = handshaker.noise.write_message(&[], &mut buf).map_err(NoiseError::from)?;
            let msg = serde_wasm_bindgen::to_value(&handshaker.buf[..len])
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))?;
            callback_down.call1(&JsValue::NULL, &msg)?;
            NoiseHandshake::SentEphemeralPublicKey
        } else {
            NoiseHandshake::WaitingForConnection
        };

        Ok(NoiseStateMachine {
            role,
            state,
            handshaker,
            up_func: callback_up,
            down_func: callback_down,
        })
    }
    
    // send mode is true if the message is to be sent to the server from the client
    #[wasm_bindgen(js_name = handleConnection)]
    pub fn handle_connection(&mut self, msg: Option<String>, send_mode: bool) -> () {
        if self.role {
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
        }else {
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

#[derive(Debug)]
pub enum NoiseError {
    SnowError(snow::Error),
    SerdeError(serde_wasm_bindgen::Error),
    OtherError(String),
}

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoiseError::SnowError(e) => write!(f, "Snow error: {}", e),
            NoiseError::SerdeError(e) => write!(f, "Snow error: {}", e),
            NoiseError::OtherError(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl std::error::Error for NoiseError {}

impl From<snow::Error> for NoiseError {
    fn from(error: snow::Error) -> NoiseError {
        NoiseError::SnowError(error)
    }
}

impl From<NoiseError> for JsValue {
    fn from(error: NoiseError) -> JsValue {
        JsValue::from_str(&error.to_string())
    }
}

// pub enum Message {
//     Text(String),
//     Binary(Vec<u8>),
// }