import { WASI } from "@wasmer/wasi";
import wasiBindings from "@wasmer/wasi/lib/bindings/browser";

let rustExports: RustrustExports;

const vecSize = 3 * 32 / 8;

function fromRustStr(ptr: number, len: number): string {
  const view = new Uint8Array(rustExports.memory.buffer, ptr, len);
  return new TextDecoder("utf-8").decode(view);
}

// function fromRustString(vec: number): string {
//   const outputStr = fromRustStr(
//     rustExports.vec_buffer(vec),
//     rustExports.vec_len(vec)
//   );
//   rustExports.vec_u8_drop(vec);
//   return outputStr;
// }

function toRustString(str: string) {
  const array = new TextEncoder().encode(str);
  const len = array.length;
  const vec = rustExports.vec_u8_with_capacity(len)
  const buf = rustExports.vec_buffer(vec)
  const view = new Uint8Array(
    rustExports.memory.buffer,
    buf,
    len
  );
  view.set(array);
  rustExports.vec_set_len(vec, len);
  return vec;
}

interface StreamWithSize {
  stream: ReadableStream<Uint8Array>;
  size: number;
}

function uploadFile(file: File): StreamWithSize {
  return { stream: file.stream(), size: file.size };
}

async function streamIntoVec(stream: StreamWithSize): Promise<number> {
  const reader = stream.stream.getReader();
  const vec = rustExports.vec_u8_with_capacity(stream.size);
  const capacity = rustExports.vec_capacity(vec);
  const vecBuffer = rustExports.vec_buffer(vec);
  try {
    let position = 0;
    for (;;) {
      const res = await reader.read();
      if (res.done) {
        break;
      }
      if (position + res.value.length > capacity) {
        throw new Error(
          `Content-Length was lower ${position} + ${res.value.length} > ${capacity}`
        );
      }
      const view = new Uint8Array(
        rustExports.memory.buffer,
        vecBuffer + position
      );
      view.set(res.value);
      position += res.value.length;
    }
    if (position != stream.size) {
      throw new Error('Content-Length was higher');
    }
    rustExports.vec_set_len(vec, position);
  } catch (e) {
    rustExports.vec_u8_drop(vec);
    throw e;
  }
  return vec;
}

function consoleTrace(ptr: number, len: number) {
  /* eslint-disable no-console */
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
  /* eslint-enable no-console */
}

interface RustrustExports {
  readonly memory: WebAssembly.Memory;
  "return_one"(): number;
  "vec_u8_with_capacity"(cap: number): number;
  "vec_vec_with_capacity"(cap: number): number;
  "vec_capacity"(vec: number): number;
  "vec_len"(vec: number): number;
  "vec_buffer"(vec: number): number;
  "vec_set_len"(vec: number, newLen: number): void;
  "vec_u8_drop"(vec: number): void;
  "jwl_merge"(inputs: number, dateNow: number): number;
  "merge_result_drop"(mergeResult: number): void;
}

export async function startWasiTask() {
  // Instantiate a new WASI Instance
  const wasi = new WASI({
    env: { RUST_BACKTRACE: "1" },
    bindings: { ...wasiBindings }
  });

  const response = fetch("./merge-jwl.wasm");
  const module = await (typeof WebAssembly.compileStreaming === "function"
    ? WebAssembly.compileStreaming(response)
    : WebAssembly.compile(await (await response).arrayBuffer()));
  const instance = await WebAssembly.instantiate(module, {
    ...wasi.getImports(module),
    env: {
      "js_console_panic": (ptr: number, len: number) =>
        // eslint-disable-next-line no-console
        console.error(fromRustStr(ptr, len)),
      "js_console_trace": consoleTrace
    }
  });
  rustExports = (instance.exports as unknown) as RustrustExports;

  // Start the WebAssembly WASI instance!
  wasi.start(instance);
  if (rustExports.return_one() !== 1) {
    throw new Error("WebAssembly failed to load");
  }
}

export async function mergeUploads(files: FileList) {
  const len = files.length;
  if (len < 2) {
    throw new Error("Merge 2 or more files");
  }
  const intputVecs = await Promise.all(Array.from(files).map(f => streamIntoVec(uploadFile(f))));
  const inputsPtr = rustExports.vec_vec_with_capacity(len);
  const inputsBuf = rustExports.vec_buffer(inputsPtr);
  new Uint32Array(
    rustExports.memory.buffer,
    inputsBuf,
    len
  ).set(intputVecs);
  rustExports.vec_set_len(inputsPtr, len);
  const filePtr = rustExports.jwl_merge(inputsPtr, toRustString(new Date().toISOString().substr(0,10)));
  if (filePtr == 0) {
    throw new Error("Merge failed");
  }
  const manifestPtr = filePtr + vecSize;
  const manifestBuf = rustExports.vec_buffer(manifestPtr);
  const manifestLen = rustExports.vec_len(manifestPtr);
  const manifest = JSON.parse(fromRustStr(manifestBuf, manifestLen));
  console.debug(manifest);
  rustExports.merge_result_drop(filePtr);
}
