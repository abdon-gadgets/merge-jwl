let rustExports: RustrustExports;

let mergeProgress: (p: Progress) => void = () => {
  return;
};

const vecSize = (3 * 32) / 8;

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
  const vec = rustExports.vec_u8_with_capacity(len);
  const buf = rustExports.vec_buffer(vec);
  const view = new Uint8Array(rustExports.memory.buffer, buf, len);
  view.set(array);
  rustExports.vec_set_len(vec, len);
  return vec;
}

interface StreamWithSize {
  stream: ReadableStream<Uint8Array>;
  size: number;
}

/** Fallback ReadableStream using FileReader */
function readerFallback(file: File) {
  // TODO: Use file.webkitSlice() to read chunks
  return new ReadableStream({
    start: function (controller) {
      const reader = new FileReader();
      reader.onload = () => {
        controller.enqueue(new Uint8Array(reader.result as ArrayBuffer));
        controller.close();
      };
      reader.onerror = () => controller.error(reader.error);
      reader.onabort = () => controller.error(new Error("FileReader aborted"));
      reader.readAsArrayBuffer(file);
    },
  });
}

function uploadFile(file: File): StreamWithSize {
  // Safari does not yet support Blob.stream() https://caniuse.com/mdn-api_blob_slice
  return {
    stream: file.stream ? file.stream() : readerFallback(file),
    size: file.size,
  };
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
      throw new Error("Content-Length was higher");
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
  "_start": () => void;
}

export const enum Progress {
  Load = 1,
  Wasm,
  Extract,
  Merge,
  Store,
  Pack,
  Js,
  Done,
}

async function _startWasi() {
  // Instantiate a new WASI Instance
  const response = fetch("/merge-jwl.wasm");
  const module = await (typeof WebAssembly.compileStreaming === "function"
    ? WebAssembly.compileStreaming(response)
    : WebAssembly.compile(await (await response).arrayBuffer()));

  const bindings = new Proxy(
    {
      "random_get": (bufPtr: number, bufLen: number) => {
        const view = new Uint8Array(rustExports.memory.buffer, bufPtr, bufLen);
        crypto.getRandomValues(view);
        console.debug("random", view);
        return 0;
      },
      "fd_prestat_get": () => 8, // ERRNO_BADF Bad file descriptor
    },
    {
      get: function <T, U>(
        target: { [prop: string]: (...rest: T[]) => U },
        prop: string
      ): (...rest: T[]) => U {
        return (...rest: T[]) => {
          const original = target[prop];
          if (original) {
            return original(...rest);
          }
          throw new Error("Unimplemented WASI " + prop.toString());
        };
      },
    }
  );

  const instance = await WebAssembly.instantiate(module, {
    "wasi_snapshot_preview1": bindings,
    env: {
      "js_console_panic": (ptr: number, len: number) =>
        // eslint-disable-next-line no-console
        console.error(fromRustStr(ptr, len)),
      "js_console_trace": consoleTrace,
      "js_merge_progress": mergeProgress,
    },
  });
  rustExports = (instance.exports as unknown) as RustrustExports;

  // Start the WebAssembly WASI instance!
  rustExports._start();
  if (rustExports.return_one() !== 1) {
    throw new Error("WebAssembly failed to load");
  }
}

let startWasiTask: undefined | Promise<void>;

export function startWasi() {
  startWasiTask = _startWasi();
}

interface ManifestJson {
  name: string;
  creationDate: string;
  version: number;
  type: number;
  userDataBackup: {
    lastModifiedDate: string;
    deviceName: string;
    databaseName: string;
    hash: string;
    schemaVersion: number;
  };
}

interface NoteText {
  title: string | null;
  content: string | null;
  date: string;
}

export interface BookmarkOverflow {
  keySymbol: string | null;
  issueTagNumber: number;
  title: string;
  snippet: string | null;
}

export interface Note {
  before: NoteText;
  after: NoteText;
}

export interface MessageJson {
  error?: string;
  noteUpdate?: Note;
  bookmarkOverflow?: BookmarkOverflow;
}

interface MergeJson {
  inputManifests: ManifestJson[];
  resultManifest: ManifestJson | null;
  messages: MessageJson[];
}

function parseResult(resultPtr: number) {
  const resultBuf = rustExports.vec_buffer(resultPtr);
  const resultLen = rustExports.vec_len(resultPtr);
  return JSON.parse(fromRustStr(resultBuf, resultLen)) as MergeJson;
}

export class Merge {
  file: File | null;
  messages: MessageJson[];
  objectURL?: string;

  constructor(filePtr: number) {
    if (filePtr == 0) {
      throw new Error("Returned null");
    }
    const mergeResult = parseResult(filePtr + vecSize);
    const fileOption = new Int32Array(rustExports.memory.buffer, filePtr, 1);
    if (fileOption[0] == 0 || !mergeResult.resultManifest) {
      this.file = null;
    } else {
      const len = rustExports.vec_len(filePtr);
      const buf = rustExports.vec_buffer(filePtr);
      const blob = new Blob([
        new Uint8Array(rustExports.memory.buffer, buf, len),
      ]);
      const fileName = mergeResult.resultManifest.name + ".jwlibrary";
      this.file = new File([blob], fileName);
      rustExports.merge_result_drop(filePtr);
    }
    this.messages = mergeResult.messages;
  }

  drop() {
    if (this.objectURL) {
      URL.revokeObjectURL(this.objectURL);
      this.objectURL = undefined;
    }
  }

  download() {
    if (this.file) {
      const a = document.createElement("a");
      a.download = this.file.name;
      a.rel = "noopener";
      this.objectURL = URL.createObjectURL(this.file);
      a.href = this.objectURL;
      a.click();
    }
  }
}

export async function mergeUploads(
  files: FileList,
  progress: (progress: Progress) => void
) {
  if (!startWasiTask) {
    throw new Error("WASI was not started");
  }
  await startWasiTask;
  mergeProgress = progress;
  const len = files.length;
  if (len < 2) {
    throw new Error("Merge 2 or more files");
  }
  mergeProgress(Progress.Load);
  const intputVecs = await Promise.all(
    Array.from(files).map((f) => streamIntoVec(uploadFile(f)))
  );
  const inputsPtr = rustExports.vec_vec_with_capacity(len);
  const inputsBuf = rustExports.vec_buffer(inputsPtr);
  new Uint32Array(rustExports.memory.buffer, inputsBuf, len).set(intputVecs);
  rustExports.vec_set_len(inputsPtr, len);
  mergeProgress(Progress.Wasm);
  await new Promise((resolve) => setTimeout(resolve, 0));
  const filePtr = rustExports.jwl_merge(
    inputsPtr,
    toRustString(new Date().toISOString().substr(0, 10))
  );
  mergeProgress(Progress.Js);
  mergeProgress = () => {
    return;
  };
  const merge = new Merge(filePtr);
  mergeProgress(Progress.Done);
  return merge;
}
