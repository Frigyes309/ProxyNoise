/* tslint:disable */
/* eslint-disable */
/**
* true for client, false for server
*/
export class NoiseStateMachine {
  free(): void;
/**
* @param {boolean} role
* @param {Function} callback_up
* @param {Function} callback_down
* @param {Uint8Array} server_static_key
*/
  constructor(role: boolean, callback_up: Function, callback_down: Function, server_static_key: Uint8Array);
/**
* send mode is true if the message is to be sent to the server from the client
* @param {Uint8Array | undefined} msg
* @param {boolean} send_mode
*/
  handleConnection(msg: Uint8Array | undefined, send_mode: boolean): void;
/**
* @returns {string}
*/
  getHandshakestate(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_noisestatemachine_free: (a: number) => void;
  readonly noisestatemachine_new_init: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly noisestatemachine_handleConnection: (a: number, b: number, c: number, d: number) => void;
  readonly noisestatemachine_getHandshakestate: (a: number, b: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
