console.log('Script is running...');
import __wbg_init, {NoiseStateMachine} from '../../pkg/potato';

console.log('Initializing WebAssembly...');

(async () => {
    try {
        const wasmModule = await __wbg_init();
        console.log('WASM module initialized:', wasmModule);

        // Now call the main function
        call();
    } catch (error) {
        console.error('Error initializing WebAssembly:', error);
    }
})();

function call() {
    const role = true;
    const callback_up = upFunction;
    const callback_down = downFunction;

    const server_static_key = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    const noise = new NoiseStateMachine(role, callback_up, callback_down, server_static_key);
    noise.handleConnection('Hello', true);
    noise.handleConnection('Hello', true);
    noise.handleConnection('Hello', true);
    noise.handleConnection('Hello', true);
    noise.free();
}

function upFunction() {
    console.log('Up function called');
}

function downFunction() {
    console.log('Down function called');
}
