import __wbg_init, {NoiseStateMachine} from '../../pkg/potato';
(async () => {
    try {
        const wasmModule = await __wbg_init();
        console.log('WASM module initialized');

        // Now call the main function
        main();
    } catch (error) {
        console.log('Error initializing WebAssembly:', error);
    }
})();

const serverUrl = 'ws://127.0.0.1:3030';
const socket = new WebSocket(serverUrl);
let noise: NoiseStateMachine;

socket.onopen = (event: Event) => {
    console.log('WebSocket connection opened');
};

socket.onmessage = async (event: MessageEvent) => {
    console.log('WebSocket message received');
    const blob = event.data as Blob;
    let data = await getUint8ArrayFromBlob(blob);
    const data2 = new Uint8Array(data);
    noise.handleConnection(data2, false);
    console.log(noise.getHandshakestate());
};

socket.onclose = (event: CloseEvent) => {
    //noise.free();
    console.log('WebSocket connection closed');
};

async function main() {
    const server_static_key = new Uint8Array([
        181, 98, 220, 5, 213, 251, 229, 176, 29, 249, 159, 24, 126, 72, 126, 146, 184, 89, 43, 181,
        155, 51, 158, 140, 90, 143, 230, 213, 30, 192, 80, 122,
    ]);
    noise = new NoiseStateMachine(true, upFunction, downFunction, server_static_key);

    send('{ "jsonrpc": "2.0", "method": "say_hello", "params": [1, 2], "id":134 }');
    await new Promise((resolve) => setTimeout(resolve, 3000));
    send('{ "jsonrpc": "2.0", "method": "say_hello", "params": [1, 2], "id":134 }');
}

function send(message: string) {
    // 1 = connection open
    if (socket.readyState === 1) {
        if (noise.getHandshakestate() == 'HandshakeCompleted') {
            console.log('Sending message:', message);
            noise.handleConnection(new TextEncoder().encode(message), true);
        } else {
            console.log('Initializing handshake');
            noise.handleConnection(undefined, true);
        }
    } else {
        console.log('WebSocket not open');
    }
}

function convertStringToBlob(str: string): Blob {
    const encoder = new TextEncoder();
    const uint8Array = encoder.encode(str);
    const blob = new Blob([uint8Array], {type: 'application/octet-stream'});
    console.log('blobbed');
    return blob;
}

async function downFunction(param: any) {
    if (param == undefined) {
    } else {
        console.log('Down function called with param:', param);
        const blob: Blob =
            typeof param === 'string'
                ? convertStringToBlob(param)
                : new Blob([new Uint8Array(param)], {type: 'application/octet-stream'});
        socket.send(blob);
        //send(param);
    }
}

function upFunction(param: any) {
    console.log('Up function called with param:', param);
}

async function getUint8ArrayFromBlob(blob: Blob): Promise<Uint8Array> {
    const arrayBuffer = await blob.arrayBuffer(); // Read the Blob as an ArrayBuffer
    const uint8Array = new Uint8Array(arrayBuffer); // Convert the ArrayBuffer to a Uint8Array
    return uint8Array; // Return the Uint8Array
}
