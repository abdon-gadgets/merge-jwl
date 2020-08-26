import { WASI } from "@wasmer/wasi";
import wasiBindings from "@wasmer/wasi/lib/bindings/browser";

let rustExports: RustrustExports;

function fromRustStr(ptr: number, len: number): string {
  const view = new Uint8Array(rustExports.memory.buffer, ptr, len);
  return new TextDecoder("utf-8").decode(view);
}

function fromRustString(vec: number): string {
  const outputStr = fromRustStr(rustExports.vec_buffer(vec), rustExports.vec_len(vec));
  rustExports.vec_drop(vec);
  return outputStr;
}

interface StreamWithSize {
  stream: ReadableStream<Uint8Array>;
  size: number;
}

async function streamIntoVec(stream: StreamWithSize): Promise<number> {
  const reader = stream.stream.getReader();
  const vec = rustExports.vec_with_capacity(stream.size);
  const capacity = rustExports.vec_capacity(vec);
  const vecBuffer = rustExports.vec_buffer(vec);
  try {
    let position = 0;
    for(;;) {
      const res = await reader.read();
      if (res.done) {
        break;
      }
      if (position + res.value.length > capacity) {
        throw new Error(
          `Content-Length was lower ${position} + ${res.value.length} > ${capacity}`
        );
      }
      const view = new Uint8Array(rustExports.memory.buffer, vecBuffer + position);
      view.set(res.value);
      position += res.value.length;
    }
    if (position != stream.size) {
      console.warn("Content-Length was higher", capacity, position);
    }
    rustExports.vec_set_len(vec, position);
    console.log("streamed to Vec ", position);
  } catch (e) {
    rustExports.vec_drop(vec);
    throw e;
  }
  return vec;
}

function consoleTrace(ptr: number, len: number) {
  const event = JSON.parse(fromRustStr(ptr, len));
  const level = (() => {
    switch (event.level) {
      case "ERROR":
        return console.error;
      case "WARN":
        return console.warn;
      case "INFO":
        return console.info;
      case "DEBUG":
      case "TRACE":
        return console.debug;
      default:
        return console.log;
    }
  })();
  delete event.timestamp;
  delete event.level;
  level("%s", event.fields.message, event);
}

interface RustrustExports {
  readonly memory: WebAssembly.Memory;
  "return_one"(): number;
  "vec_with_capacity"(cap: number): number;
  "vec_capacity"(vec: number): number;
  "vec_len"(vec: number): number;
  "vec_buffer"(vec: number): number;
  "vec_set_len"(vec: number, newLen: number): void;
  "vec_drop"(vec: number): void;
  "jwl_merge"(inputs: number, dateNow: number): void;
}

export async function startWasiTask() {
  // Instantiate a new WASI Instance
const wasi = new WASI({
  env: { RUST_BACKTRACE: "1" },
  bindings: { ...wasiBindings },
});

const response = fetch("./merge-jwl.wasm");
const module = await (typeof WebAssembly.compileStreaming === "function"
  ? WebAssembly.compileStreaming(response)
  : WebAssembly.compile(await (await response).arrayBuffer()));
console.debug("WebAssembly compiled");
const instance = await WebAssembly.instantiate(module, {
  ...wasi.getImports(module),
  env: {
    "js_console_panic": (ptr: number, len: number) =>
      console.error(fromRustStr(ptr, len)),
    "js_console_trace": consoleTrace,
  },
});
rustExports = instance.exports as unknown as RustrustExports;

// Start the WebAssembly WASI instance!
wasi.start(instance);
console.debug("check " + rustExports.return_one());
}
