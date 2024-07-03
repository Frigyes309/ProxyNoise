/*const WebSocket = require('rpc-websockets').Client;
const url = require('url');

// Create a new WebSocket client instance
const ws = new WebSocket('ws://localhost:3030'); // Replace with your RPC server URL

let requestId = 1;

// Connect to the RPC server
ws.on('open', () => {
    console.log('Connected to the RPC server');

    // Example URL for testing
    const testUrl = 'http://localhost:3030/add/1/2';

    // Parse the URL to get the path
    const parsedUrl = url.parse(testUrl);
    const pathParts = parsedUrl.pathname.split('/').filter(Boolean); // Remove empty parts

    if (pathParts.length >= 2) {
        const methodName = pathParts[0]; // First part is the method name
        const params = pathParts.slice(1); // Remaining parts are the parameters

        // Create the JSON-RPC request
        const jsonRpcRequest = {
            jsonrpc: '2.0',
            method: methodName,
            params: params,
            id: requestId++,
        };

        // Call the RPC method with the parameters
        ws.call(methodName, params)
            .then((result) => {
                console.log('RPC method result:', result);
                // Optionally close the connection after receiving the response
                // ws.close();
            })
            .catch((error) => {
                console.error('Error calling RPC method:', error);
                // Optionally close the connection if there's an error
                // ws.close();
            });

        console.log('JSON-RPC request:', jsonRpcRequest);
    } else {
        console.error('Invalid URL format. Expected format: /methodName/param1/param2');
    }
});

// Handle errors
ws.on('error', (error) => {
    console.error('WebSocket error:', error);
});

// Handle connection close
ws.on('close', () => {
    console.log('Connection closed');
});


const WebSocket = require('ws');

// Connect to the WebSocket server
const ws = new WebSocket('ws://127.0.0.1:3030');

// Define the JSON-RPC request payload
const requestPayload = {
    jsonrpc: '2.0',
    method: 'say_hello',
    params: [1, 2],
    id: 134,
};

// Convert the payload to a JSON string
const requestString = JSON.stringify(requestPayload);

// Event listener for when the connection is established
ws.on('open', function open() {
    console.log('Connected to the WebSocket server');
    // Send the JSON-RPC request
    setTimeout(() => {
        ws.send(requestString);
        console.log('Request sent:', requestString);
    }, 19000);
    console.log('Request sent:', requestString);
});

// Event listener for when a message is received from the server
ws.on('message', function incoming(data) {
    console.log('Received message from server:', data);
});

// Event listener for handling connection errors
ws.on('error', function error(err) {
    console.error('WebSocket error:', err);
});

// Event listener for when the connection is closed
ws.on('close', function close() {
    console.log('WebSocket connection closed');
});
*/
const WebSocket = require('ws');
const {Noise} = require('@stablelib/noise');
const {randomBytes} = require('@stablelib/random');
const {readFileSync} = require('fs');

const IP_PORT = 'your_ip:port'; // Replace with your IP and port
const NOISE_PARAMS = 'Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s'; // Replace with your Noise protocol parameters
const SECRET = 'Random 32 characters long secret'; // Replace with your secret key

let requestId = 1;

const payloadGenerator = () => {
    // Implement your payload generation logic here
    const requestPayload = {
        jsonrpc: '2.0',
        method: 'say_hello',
        params: [1, 2],
        id: requestId++,
    };
    return requestPayload;
};

const startWebSocketClient = async () => {
    try {
        const url = `ws://127.0.0.1:3030`;
        const ws = new WebSocket(url);

        ws.on('open', async () => {
            const builder = new Noise(NOISE_PARAMS);
            const staticKey = builder.generateKeyPair().privateKey;
            let noise = builder
                .setLocalPrivateKey(staticKey)
                .setPSK(3, SECRET)
                .initializeInitiator();

            const buf = new Uint8Array(65535);

            // -> e
            const len = noise.writeMessage(new Uint8Array(0), buf);
            ws.send(buf.slice(0, len));

            // <- e, ee, s, es
            ws.on('message', async (data) => {
                noise.readMessage(data, buf);

                // -> s, se
                const len1 = noise.writeMessage(new Uint8Array(0), buf);
                ws.send(buf.slice(0, len1));

                noise = noise.split();
                console.log('Session established...');

                let msg = payloadGenerator();
                let len2 = noise.writeMessage(Buffer.from(msg, 'utf-8'), buf);
                ws.send(buf.slice(0, len2));

                ws.on('message', async (data) => {
                    try {
                        const len = noise.readMessage(data, buf);
                        const response = new TextDecoder().decode(buf.slice(0, len));
                        console.log('Server said:', response);
                        const msgJson = JSON.parse(response);
                        const isExit = msgJson.result === 'exit';

                        if (isExit) {
                            ws.close();
                            console.log('Connection closed.');
                            return;
                        }

                        msg = payloadGenerator();
                        len = noise.writeMessage(Buffer.from(msg, 'utf-8'), buf);
                        ws.send(buf.slice(0, len));
                    } catch (e) {
                        console.error('Error:', e);
                        ws.close();
                    }
                });
            });
        });

        ws.on('error', (e) => {
            console.error('WebSocket error:', e);
        });

        ws.on('close', () => {
            console.log('Connection closed.');
        });
    } catch (e) {
        console.error('Failed to connect:', e);
    }
};

startWebSocketClient();
