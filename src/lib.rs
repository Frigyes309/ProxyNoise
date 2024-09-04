use lazy_static::lazy_static;
use snow::{params::NoiseParams, HandshakeState, TransportState};
use wasm_bindgen::prelude::*;
use snow::Builder;
use crate::NoiseHandshake::{SentEphemeralPublicKey, WaitingForConnection};
use NoiseHandshake::*;
use std::fmt;

// type ResultHandler = Result<(NoiseHandshake, JsValue), anyhow::Error>;
type ResultHandler = Result<JsValue, NoiseError>;

lazy_static! {
    static ref NOISE_PARAMS: NoiseParams = "Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s"
        .parse()
        .expect("Parsing a constant will cause no error");
}

#[derive(Debug)]
enum NoiseHandshake {
    WaitingForConnection,
    ReceivedPublicStaticKey,
    SentEphemeralPublicKey,
    ReceivedEphemeralPublicKey,
    SentStaticPublicKey,
    HandshakeCompleted,
}

/// true for client, false for server
#[wasm_bindgen(js_name = NoiseStateMachine)]
struct NoiseStateMachine {
    role: bool, 
    state: NoiseHandshake,
    handshaker: Handshaker,
    up_func: js_sys::Function,
    down_func: js_sys::Function,
}

trait NoiseClient {
    fn initiator_start_handshake(&mut self) -> ResultHandler;
    fn initiator_second_phase(&mut self, msg: Vec<u8>) -> ResultHandler;
    fn initiator_third_phase(&mut self) -> ResultHandler;
    fn initiator_send_request(&mut self) -> ResultHandler;
    fn initiator_handle_response(&mut self, msg: Vec<u8>) -> ResultHandler;
}

struct Handshaker {
    noise: HandshakeState,
    buf: Vec<u8>,
}

impl NoiseClient for Handshaker {
    fn initiator_start_handshake(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(x) => x,
            Err(e) => NoiseError::OtherError(format!("Serialization error: {:?}", e)).into(),
        };
        Ok(msg)
    }

    fn initiator_second_phase(&mut self, msg: Vec<u8>) -> ResultHandler {
        self.noise.read_message(&msg, &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value("") {
            Ok(msg) => msg,
            Err(e) => NoiseError::OtherError(format!("Serialization error: {:?}", e)).into(),
        };
        Ok(msg)
    }

    fn initiator_third_phase(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => NoiseError::OtherError(format!("Serialization error: {:?}", e)).into(),
        };
        Ok(msg)
    }

    fn initiator_send_request(&mut self) -> ResultHandler {
        let len = self.noise.write_message(&[], &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => NoiseError::OtherError(format!("Serialization error: {:?}", e)).into(),
        };
        Ok(msg)
    }

    fn initiator_handle_response(&mut self, msg: Vec<u8>) -> ResultHandler {
        let len = self.noise.read_message(&msg, &mut self.buf)?;
        let msg = match serde_wasm_bindgen::to_value(&self.buf[..len]) {
            Ok(msg) => msg,
            Err(e) => NoiseError::OtherError(format!("Serialization error: {:?}", e)).into(),
        };
        Ok(msg)
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
        let builder = Builder::new(NOISE_PARAMS.clone())
            .remote_public_key(&server_static_key);
        let static_key = builder.generate_keypair().map_err(NoiseError::from)?.private;
        let secret: [u8; 32] = *b"Random 32 characters long secret";
        let noise: HandshakeState = builder
            .local_private_key(&static_key)
            .psk(3, &secret)
            .build_initiator().map_err(NoiseError::from)?;
        let buf: Vec<u8> = vec![0u8; 65535];
        let handshaker = Handshaker {
            noise,
            buf: buf.clone(),
        };

        Ok(NoiseStateMachine {
            role,
            state: NoiseHandshake::WaitingForConnection,
            handshaker,
            up_func: callback_up,
            down_func: callback_down,
        })
    }

    /// send mode is true if the message is to be sent to the server from the client
    #[wasm_bindgen(js_name = handleConnection)]
    pub fn handle_connection(&mut self, msg: Option<Vec<u8>>, send_mode: bool) {
        if self.role {
            match self.state {
                WaitingForConnection => {
                    let msg = match self.handshaker.initiator_start_handshake() {
                        Ok(msg) => msg,
                        Err(e) => NoiseError::InitiationSendError(format!("Error: {:?}", e)).into(),
                    };
                    let _ = self.down_func.call1(&JsValue::NULL, &msg);
                    self.state = SentEphemeralPublicKey;
                }
                SentEphemeralPublicKey => {
                    let msg = match msg {
                        Some(msg) => msg,
                        None => NoiseError::OtherError("No message received".to_string()).into(),
                    };
                    let _msg = match self.handshaker.initiator_second_phase(msg) {
                        Ok(msg) => msg,
                        Err(e) => NoiseError::InitiationSendError(format!("Error: {:?}", e)).into(),
                    };
                    self.state = ReceivedEphemeralPublicKey;
                    self.handle_connection(None, true);
                }
                ReceivedEphemeralPublicKey => {
                    let msg = match self.handshaker.initiator_third_phase() {
                        Ok(msg) => msg,
                        Err(e) => NoiseError::InitiationSendError(format!("Error: {:?}", e)).into(),
                    };
                    let _ = self.down_func.call1(&JsValue::NULL, &msg);
                    self.state = HandshakeCompleted;
                    // self.handshaker.transport = self.handshaker.noise.into_transport_mode();
                }
                HandshakeCompleted => {
                    if send_mode {
                        let msg = match self.handshaker.initiator_send_request() {
                            Ok(msg) => msg,
                            Err(e) => NoiseError::OtherError(format!("Error: {:?}", e)).into(),
                        };
                        let _ = self.down_func.call1(&JsValue::NULL, &msg);
                    } else {
                        let msg = match msg {
                            Some(msg) => msg,
                            None => NoiseError::OtherError(format!("No message received$")).into(),
                        };
                        let msg =
                            match self.handshaker.initiator_handle_response(msg) {
                                Ok(msg) => msg,
                                Err(e) => NoiseError::OtherError(format!("Error: {:?}", e)).into(),
                            };
                        let _ = self.up_func.call1(&JsValue::NULL, &msg);
                    }
                }
                _ => {
                    // let error_msg = NoiseError::InvalidStateError("Invalid state".to_string()).to_string();
                    // eprint!(&error_msg);
                }
            }
        } else {
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

    #[wasm_bindgen(js_name = getHandshakestate)]
    pub fn getHandshakestate(&self) -> String {
        format!("{:?}", self.state)
    }
}

#[derive(Debug)]
pub enum NoiseError {
    SnowError(snow::Error),
    SerdeError(serde_wasm_bindgen::Error),
    InitiationSendError(String),
    OtherError(String),
    InvalidStateError(String),
}

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoiseError::SnowError(e) => write!(f, "Snow error: {}", e),
            NoiseError::SerdeError(e) => write!(f, "Snow error: {}", e),
            NoiseError::InitiationSendError(e) => write!(f, "Initiation error: {:?}", e),
            NoiseError::InvalidStateError(e) => write!(f, "Invalid state error: {}", e),
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

impl From<JsValue> for NoiseError {
    fn from(error: JsValue) -> NoiseError {
        NoiseError::InitiationSendError(format!("{:?}", error))
    }
}

impl Into<Vec<u8>> for NoiseError {
    fn into(self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }
}

impl From<NoiseError> for JsValue {
    fn from(error: NoiseError) -> JsValue {
        JsValue::from_str(&error.to_string())
    }
}

//     Text(String),
//     Binary(Vec<u8>),
// }