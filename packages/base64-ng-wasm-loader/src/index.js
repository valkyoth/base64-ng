const getPrototypeOf = Reflect.getPrototypeOf;
const getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
const call = Function.prototype.call.bind(Function.prototype.call);
const TypedArrayPrototype = getPrototypeOf(Uint8Array.prototype);
const typedArrayTag = getOwnPropertyDescriptor(TypedArrayPrototype, Symbol.toStringTag).get;
const typedArrayBuffer = getOwnPropertyDescriptor(TypedArrayPrototype, "buffer").get;
const typedArrayByteLength = getOwnPropertyDescriptor(TypedArrayPrototype, "byteLength").get;
const typedArrayByteOffset = getOwnPropertyDescriptor(TypedArrayPrototype, "byteOffset").get;
const typedArrayLength = getOwnPropertyDescriptor(TypedArrayPrototype, "length").get;
const uint8Subarray = Uint8Array.prototype.subarray;
const arrayBufferByteLength = getOwnPropertyDescriptor(ArrayBuffer.prototype, "byteLength").get;
const arrayBufferResizable = getOwnPropertyDescriptor(ArrayBuffer.prototype, "resizable")?.get;
const arrayBufferMaxByteLength = getOwnPropertyDescriptor(ArrayBuffer.prototype, "maxByteLength")?.get;
const sharedArrayBufferByteLength = typeof SharedArrayBuffer === "undefined"
  ? undefined
  : getOwnPropertyDescriptor(SharedArrayBuffer.prototype, "byteLength").get;
const uint8Set = Uint8Array.prototype.set;
const stringCharCodeAt = String.prototype.charCodeAt;
const IntrinsicUint8Array = Uint8Array;
const EMPTY_BYTES = new IntrinsicUint8Array(0);
const numberIsSafeInteger = Number.isSafeInteger;
const numberFrom = Number;
const wasmValidate = WebAssembly.validate.bind(WebAssembly);
const wasmInstantiate = WebAssembly.instantiate.bind(WebAssembly);
const cryptoSubtle = globalThis.crypto?.subtle;
const cryptoDigest = cryptoSubtle?.digest;

const SIMD_PROBE = new IntrinsicUint8Array([
  0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123,
  3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15, 253, 98, 11,
]);

const ABI_ERRORS = Object.freeze({
  1: "invalid-codec",
  2: "input-limit",
  3: "invalid-input",
  4: "invalid-length",
  5: "invalid-byte",
  6: "invalid-padding",
  7: "output-capacity",
  8: "backend-failure",
});

const NO_INDEX = 0xffff_ffff;
const HARD_MAX_MEMORY_PAGES = 128;
const MAX_ARTIFACT_BYTES = 1024 * 1024;
const ARTIFACT_SHA256 = Object.freeze({
  scalar: "1267b3de661aa478250eed1346de98369f76ef495a9fe7f877c41ea32f26f4ed",
  simd128: "855575b093ab757b8a07f124cfc7f92b1c14fd9f53b8c7055c3b3a6fb20332b6",
});

export const Codecs = Object.freeze({
  STANDARD: "standard",
  STANDARD_NO_PAD: "standard-no-pad",
  URL_SAFE: "url-safe",
  URL_SAFE_NO_PAD: "url-safe-no-pad",
});

export class Base64NgError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "Base64NgError";
    Object.defineProperties(this, {
      code: { value: code, enumerable: true },
      index: { value: details.index, enumerable: details.index !== undefined },
      required: { value: details.required, enumerable: details.required !== undefined },
      limit: { value: details.limit, enumerable: details.limit !== undefined },
    });
  }
}

export async function createBase64Ng(options = {}) {
  if (arrayBufferResizable === undefined || arrayBufferMaxByteLength === undefined) {
    throw error(
      "runtime-intrinsics-unavailable",
      "the runtime cannot prove fixed ArrayBuffer backing storage",
    );
  }
  const maxInputLength = exactLimit(options.maxInputLength ?? 1024 * 1024, "maxInputLength");
  const maxOutputLength = exactLimit(
    options.maxOutputLength ?? Math.ceil(maxInputLength / 3) * 4,
    "maxOutputLength",
  );
  const maximumMemoryPages = exactLimit(
    options.maximumMemoryPages ?? HARD_MAX_MEMORY_PAGES,
    "maximumMemoryPages",
  );
  if (maximumMemoryPages > HARD_MAX_MEMORY_PAGES) {
    throw error("memory-limit", "maximumMemoryPages exceeds the artifact ceiling", {
      limit: HARD_MAX_MEMORY_PAGES,
    });
  }

  const deployment = options.artifact ?? "auto";
  if (deployment !== "auto" && deployment !== "scalar" && deployment !== "simd128") {
    throw error("invalid-artifact-policy", "artifact must be auto, scalar, or simd128");
  }
  const simdValidated = wasmValidate(SIMD_PROBE);
  if (deployment === "simd128" && !simdValidated) {
    throw error("simd-unavailable", "the runtime rejected the embedded simd128 probe");
  }
  const selectedArtifact = deployment === "scalar" || !simdValidated ? "scalar" : "simd128";
  const selectionReason = deployment === "scalar"
    ? "deployment-forced-scalar"
    : deployment === "simd128"
      ? "deployment-required-simd128"
      : simdValidated
        ? "embedded-simd128-probe-validated"
        : "embedded-simd128-probe-rejected";

  const configuredSource = selectedArtifact === "simd128"
    ? options.simdArtifact
    : options.scalarArtifact;
  const configuredDigest = selectedArtifact === "simd128"
    ? options.simdArtifactSha256
    : options.scalarArtifactSha256;
  const customArtifact = configuredSource !== undefined;
  const source = customArtifact
    ? configuredSource
    : new URL(`../artifacts/base64-ng-${selectedArtifact}.wasm`, import.meta.url);
  const expectedDigest = customArtifact
    ? exactSha256(configuredDigest)
    : ARTIFACT_SHA256[selectedArtifact];
  const bytes = await loadArtifactBytes(source);
  await verifyArtifact(bytes, expectedDigest);
  const { instance } = await wasmInstantiate(bytes, {});
  const exports = validateExports(instance.exports, selectedArtifact);
  const initialPages = checkedWasmInteger(exports.memory.buffer.byteLength / 65536, "memory pages");
  if (initialPages > maximumMemoryPages) {
    throw error("memory-limit", "artifact initial memory exceeds maximumMemoryPages", {
      required: initialPages,
      limit: maximumMemoryPages,
    });
  }
  const artifactInputCapacity = checkedWasmInteger(exports.base64_ng_input_capacity(), "input capacity");
  const artifactOutputCapacity = checkedWasmInteger(exports.base64_ng_output_capacity(), "output capacity");
  if (maxInputLength > artifactInputCapacity || maxOutputLength > artifactOutputCapacity) {
    exports.base64_ng_clear();
    throw error("artifact-capacity", "configured limits exceed artifact capacity", {
      required: Math.max(maxInputLength, maxOutputLength),
      limit: Math.min(artifactInputCapacity, artifactOutputCapacity),
    });
  }

  let disposed = false;
  const posture = Object.freeze({
    capabilityEvidence: simdValidated ? "embedded-probe-validated" : "embedded-probe-rejected",
    selectedArtifact,
    selectionReason,
    artifactPosture: exports.base64_ng_artifact_posture() === 1 ? "simd128" : "scalar",
    artifactSha256: expectedDigest,
    abiVersion: exports.base64_ng_abi_version(),
    maxInputLength,
    maxOutputLength,
    maximumMemoryPages,
    maxArtifactBytes: MAX_ARTIFACT_BYTES,
    secretApi: false,
  });
  if (posture.artifactPosture !== selectedArtifact) {
    exports.base64_ng_clear();
    throw error("artifact-mismatch", "selected artifact reported a different compile-time posture");
  }

  function requireActive() {
    if (disposed) throw error("disposed", "base64-ng wasm instance has been disposed");
    const pages = checkedWasmInteger(exports.memory.buffer.byteLength / 65536, "memory pages");
    if (pages > maximumMemoryPages) {
      disposed = true;
      throw error("memory-limit", "linear memory exceeded the configured ceiling", {
        required: pages,
        limit: maximumMemoryPages,
      });
    }
  }

  function prepareInput(value) {
    requireActive();
    const inspected = inspectUint8Array(value, "input");
    if (inspected.length > maxInputLength) {
      throw error("input-limit", "input exceeds maxInputLength", { limit: maxInputLength });
    }
    return snapshotUint8Array(value, inspected);
  }

  function prepareInto(input, destination) {
    requireActive();
    const inputInfo = inspectUint8Array(input, "input");
    const destinationInfo = inspectUint8Array(destination, "destination");
    if (inputInfo.length > maxInputLength) {
      throw error("input-limit", "input exceeds maxInputLength", { limit: maxInputLength });
    }
    if (rangesOverlap(inputInfo, destinationInfo)) {
      throw error("overlapping-views", "input and destination views overlap");
    }
    return { snapshot: snapshotUint8Array(input, inputInfo), destination, destinationInfo };
  }

  function copyInput(snapshot) {
    requireActive();
    const pointer = checkedWasmInteger(exports.base64_ng_input_ptr(), "input pointer");
    const memory = new IntrinsicUint8Array(exports.memory.buffer, pointer, snapshot.length);
    call(uint8Set, memory, snapshot, 0);
  }

  function invoke(snapshot, operation, codec) {
    copyInput(snapshot);
    try {
      const result = codec === undefined ? operation(snapshot.length) : operation(snapshot.length, codec);
      if (!numberIsSafeInteger(result) || result < 0) throwWasmError(exports);
      return checkedWasmInteger(result, "operation result");
    } catch (caught) {
      if (caught instanceof Base64NgError) throw caught;
      throw error("wasm-exception", "WebAssembly operation failed without input disclosure");
    }
  }

  function outputCopy(length) {
    requireActive();
    const pointer = checkedWasmInteger(exports.base64_ng_output_ptr(), "output pointer");
    const source = new IntrinsicUint8Array(exports.memory.buffer, pointer, length);
    const copy = new IntrinsicUint8Array(length);
    call(uint8Set, copy, source, 0);
    return copy;
  }

  function clearScratch(inputUsed, outputUsed) {
    exports.base64_ng_clear_used(inputUsed, outputUsed);
  }

  function checkedOutputLength(length) {
    if (length > maxOutputLength) {
      throw error("output-limit", "output exceeds maxOutputLength", {
        required: length,
        limit: maxOutputLength,
      });
    }
    return length;
  }

  function encodedLength(input, codec = Codecs.STANDARD) {
    const snapshot = prepareInput(input);
    const id = codecId(codec);
    try {
      return checkedOutputLength(invoke(snapshot, exports.base64_ng_encoded_length, id));
    } finally {
      clearScratch(snapshot.length, 0);
    }
  }

  function decodedLength(input, codec = Codecs.STANDARD) {
    const snapshot = prepareInput(input);
    const id = codecId(codec);
    try {
      return checkedOutputLength(invoke(snapshot, exports.base64_ng_decoded_length, id));
    } finally {
      clearScratch(snapshot.length, 0);
    }
  }

  function transform(input, codec, lengthOperation, operation) {
    const snapshot = prepareInput(input);
    const id = codecId(codec);
    let outputUsed = 0;
    try {
      const expected = checkedOutputLength(invoke(snapshot, lengthOperation, id));
      const written = invoke(snapshot, operation, id);
      outputUsed = written;
      if (written !== expected) throw error("backend-failure", "WASM length contract diverged");
      return outputCopy(written);
    } finally {
      clearScratch(snapshot.length, outputUsed);
    }
  }

  function transformInto(input, destination, codec, lengthOperation, operation) {
    const prepared = prepareInto(input, destination);
    const id = codecId(codec);
    let outputUsed = 0;
    try {
      const expected = checkedOutputLength(invoke(prepared.snapshot, lengthOperation, id));
      if (prepared.destinationInfo.length < expected) {
        throw error("output-capacity", "destination is too small", { required: expected });
      }
      const written = invoke(prepared.snapshot, operation, id);
      outputUsed = written;
      if (written !== expected) throw error("backend-failure", "WASM length contract diverged");
      const pointer = checkedWasmInteger(exports.base64_ng_output_ptr(), "output pointer");
      const staged = new IntrinsicUint8Array(exports.memory.buffer, pointer, written);
      call(uint8Set, prepared.destination, staged, 0);
      return written;
    } finally {
      clearScratch(prepared.snapshot.length, outputUsed);
    }
  }

  function decodeForgiving(input) {
    const snapshot = prepareInput(input);
    let outputUsed = 0;
    try {
      const expected = checkedOutputLength(
        invoke(snapshot, exports.base64_ng_decoded_length_forgiving),
      );
      const written = invoke(snapshot, exports.base64_ng_decode_forgiving);
      outputUsed = written;
      if (written !== expected) throw error("backend-failure", "WASM length contract diverged");
      return outputCopy(written);
    } finally {
      clearScratch(snapshot.length, outputUsed);
    }
  }

  function dispose() {
    if (!disposed) {
      exports.base64_ng_clear();
      disposed = true;
    }
  }

  return Object.freeze({
    posture,
    encodedLength,
    decodedLength,
    encode: (input, codec = Codecs.STANDARD) => transform(
      input, codec, exports.base64_ng_encoded_length, exports.base64_ng_encode,
    ),
    decode: (input, codec = Codecs.STANDARD) => transform(
      input, codec, exports.base64_ng_decoded_length, exports.base64_ng_decode,
    ),
    encodeInto: (input, destination, codec = Codecs.STANDARD) => transformInto(
      input, destination, codec, exports.base64_ng_encoded_length, exports.base64_ng_encode,
    ),
    decodeInto: (input, destination, codec = Codecs.STANDARD) => transformInto(
      input, destination, codec, exports.base64_ng_decoded_length, exports.base64_ng_decode,
    ),
    decodeForgiving,
    dispose,
  });
}

function inspectUint8Array(value, label) {
  let tag;
  let buffer;
  let byteLength;
  let byteOffset;
  let length;
  try {
    tag = call(typedArrayTag, value);
    buffer = call(typedArrayBuffer, value);
    byteLength = call(typedArrayByteLength, value);
    byteOffset = call(typedArrayByteOffset, value);
    length = call(typedArrayLength, value);
  } catch {
    throw error("invalid-byte-view", `${label} must be a genuine Uint8Array`);
  }
  if (tag !== "Uint8Array") {
    throw error("invalid-byte-view", `${label} must be a genuine Uint8Array`);
  }
  if (sharedArrayBufferByteLength !== undefined) {
    try {
      call(sharedArrayBufferByteLength, buffer);
      throw error("shared-buffer", `${label} must not use SharedArrayBuffer`);
    } catch (caught) {
      if (caught instanceof Base64NgError) throw caught;
    }
  }
  try {
    call(arrayBufferByteLength, buffer);
    if (arrayBufferResizable !== undefined && call(arrayBufferResizable, buffer)) {
      throw error("resizable-buffer", `${label} must use fixed ArrayBuffer storage`);
    }
    if (arrayBufferMaxByteLength !== undefined) call(arrayBufferMaxByteLength, buffer);
  } catch (caught) {
    if (caught instanceof Base64NgError) throw caught;
    throw error("detached-buffer", `${label} backing storage is detached or unprovable`);
  }
  for (const [name, valueToCheck] of [["byteLength", byteLength], ["byteOffset", byteOffset], ["length", length]]) {
    if (!numberIsSafeInteger(valueToCheck) || valueToCheck < 0) {
      throw error("invalid-byte-view", `${label} ${name} is not an exact safe integer`);
    }
  }
  if (byteLength !== length || byteOffset + byteLength > call(arrayBufferByteLength, buffer)) {
    throw error("invalid-byte-view", `${label} has inconsistent byte bounds`);
  }
  try {
    call(uint8Set, value, EMPTY_BYTES, 0);
  } catch {
    throw error("detached-buffer", `${label} backing storage is detached`);
  }
  return { buffer, byteLength, byteOffset, length };
}

function snapshotUint8Array(value, before, label = "input") {
  const snapshot = new IntrinsicUint8Array(before.length);
  try {
    call(uint8Set, snapshot, value, 0);
  } catch {
    throw error("detached-buffer", `${label} became detached before snapshot completion`);
  }
  const after = inspectUint8Array(value, label);
  if (before.buffer !== after.buffer || before.byteLength !== after.byteLength
      || before.byteOffset !== after.byteOffset || before.length !== after.length) {
    throw error("unstable-byte-view", `${label} storage changed during snapshot`);
  }
  return snapshot;
}

function rangesOverlap(left, right) {
  if (left.buffer !== right.buffer || left.byteLength === 0 || right.byteLength === 0) return false;
  const leftEnd = left.byteOffset + left.byteLength;
  const rightEnd = right.byteOffset + right.byteLength;
  return left.byteOffset < rightEnd && right.byteOffset < leftEnd;
}

function codecId(codec) {
  if (typeof codec !== "string") {
    throw error("invalid-codec", "Base64 codec selection must be a string constant");
  }
  switch (codec) {
    case Codecs.STANDARD: return 0;
    case Codecs.STANDARD_NO_PAD: return 1;
    case Codecs.URL_SAFE: return 2;
    case Codecs.URL_SAFE_NO_PAD: return 3;
    default: throw error("invalid-codec", "unknown Base64 codec");
  }
}

function exactLimit(value, name) {
  if (!numberIsSafeInteger(value) || value < 0 || value > 0xffff_ffff) {
    throw error("invalid-limit", `${name} must be an exact non-negative WASM32 integer`);
  }
  return value;
}

function exactSha256(value) {
  if (typeof value !== "string" || value.length !== 64) {
    throw error(
      "artifact-integrity-policy",
      "custom WASM artifacts require an exact lowercase SHA-256 digest",
    );
  }
  for (let index = 0; index < value.length; index += 1) {
    const code = call(stringCharCodeAt, value, index);
    const digit = code >= 48 && code <= 57;
    const lowerHex = code >= 97 && code <= 102;
    if (!digit && !lowerHex) {
      throw error(
        "artifact-integrity-policy",
        "custom WASM artifacts require an exact lowercase SHA-256 digest",
      );
    }
  }
  return value;
}

async function verifyArtifact(bytes, expectedDigest) {
  if (cryptoSubtle === undefined || typeof cryptoDigest !== "function") {
    throw error("runtime-intrinsics-unavailable", "SHA-256 verification is unavailable");
  }
  let digestBuffer;
  try {
    digestBuffer = await call(cryptoDigest, cryptoSubtle, "SHA-256", bytes);
  } catch {
    throw error("artifact-integrity", "WASM artifact SHA-256 verification failed");
  }
  const digest = new IntrinsicUint8Array(digestBuffer);
  if (digest.length !== 32) {
    throw error("artifact-integrity", "WASM artifact SHA-256 verification failed");
  }
  for (let index = 0; index < digest.length; index += 1) {
    const upper = hexNibble(call(stringCharCodeAt, expectedDigest, index * 2));
    const lower = hexNibble(call(stringCharCodeAt, expectedDigest, index * 2 + 1));
    if (digest[index] !== (upper << 4 | lower)) {
      throw error("artifact-integrity", "WASM artifact digest mismatch");
    }
  }
}

function hexNibble(code) {
  return code <= 57 ? code - 48 : code - 87;
}

function checkedWasmInteger(value, name) {
  if (!numberIsSafeInteger(value) || value < 0 || value > 0xffff_ffff) {
    throw error("invalid-wasm-result", `${name} is outside the supported WASM32 range`);
  }
  return value;
}

function throwWasmError(exports) {
  const numericCode = exports.base64_ng_last_error_code();
  const code = ABI_ERRORS[numericCode] ?? "unknown-wasm-error";
  // WebAssembly i32 results arrive in JavaScript as signed numbers. Interpret
  // ABI fields as their documented u32 representation before sentinel checks.
  const index = exports.base64_ng_last_error_index() >>> 0;
  const required = exports.base64_ng_last_error_required() >>> 0;
  throw error(code, `base64-ng WASM operation failed: ${code}`, {
    index: index === NO_INDEX ? undefined : index,
    required: required === 0 ? undefined : required,
  });
}

function validateExports(exports, selectedArtifact) {
  const required = [
    "memory", "base64_ng_abi_version", "base64_ng_artifact_posture",
    "base64_ng_input_ptr", "base64_ng_input_capacity", "base64_ng_output_ptr",
    "base64_ng_output_capacity", "base64_ng_last_error_code",
    "base64_ng_last_error_index", "base64_ng_last_error_required",
    "base64_ng_encoded_length", "base64_ng_decoded_length",
    "base64_ng_decoded_length_forgiving", "base64_ng_encode", "base64_ng_decode",
    "base64_ng_decode_forgiving", "base64_ng_clear", "base64_ng_clear_used",
  ];
  for (const name of required) {
    if (!(name in exports)) throw error("artifact-abi", `${selectedArtifact} artifact lacks ${name}`);
  }
  if (exports.base64_ng_abi_version() !== 1) {
    throw error("artifact-abi", "unsupported base64-ng WASM ABI version");
  }
  return exports;
}

async function loadArtifactBytes(source) {
  if (isGenuineUint8Array(source)) {
    const inspected = inspectUint8Array(source, "artifact");
    enforceArtifactSize(inspected.length);
    return snapshotUint8Array(source, inspected, "artifact");
  }
  const url = source instanceof URL ? source : new URL(source, import.meta.url);
  if (url.protocol === "file:" && typeof process !== "undefined" && process.versions?.node) {
    return readNodeArtifact(url);
  }
  return readHttpArtifact(url);
}

async function readNodeArtifact(url) {
  const { open } = await import("node:fs/promises");
  let file;
  try {
    file = await open(url, "r");
  } catch {
    throw error("artifact-fetch", "WASM artifact file could not be opened");
  }
  const staged = new IntrinsicUint8Array(MAX_ARTIFACT_BYTES + 1);
  let total = 0;
  try {
    while (total < staged.length) {
      const result = await file.read(staged, total, staged.length - total, total);
      if (result.bytesRead === 0) break;
      total += result.bytesRead;
    }
  } catch {
    throw error("artifact-fetch", "WASM artifact file could not be read");
  } finally {
    await file.close();
  }
  enforceArtifactSize(total);
  return exactArtifactPrefix(staged, total);
}

async function readHttpArtifact(url) {
  let response;
  try {
    response = await fetch(url);
  } catch {
    throw error("artifact-fetch", "WASM artifact fetch failed");
  }
  if (!response.ok) throw error("artifact-fetch", `artifact fetch failed with status ${response.status}`);
  if (response.body === null || typeof response.body.getReader !== "function") {
    throw error("artifact-stream", "WASM artifact response is not a bounded readable stream");
  }
  const reader = response.body.getReader();
  const declaredText = response.headers.get("content-length");
  if (declaredText !== null) {
    const declared = numberFrom(declaredText);
    if (numberIsSafeInteger(declared) && declared > MAX_ARTIFACT_BYTES) {
      await cancelArtifactReader(reader);
      throw artifactSizeError(declared);
    }
  }

  const staged = new IntrinsicUint8Array(MAX_ARTIFACT_BYTES + 1);
  let total = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      const inspected = inspectUint8Array(value, "artifact stream chunk");
      if (inspected.length > MAX_ARTIFACT_BYTES - total) {
        await cancelArtifactReader(reader);
        throw artifactSizeError(total + inspected.length);
      }
      call(uint8Set, staged, value, total);
      total += inspected.length;
    }
  } catch (caught) {
    if (caught instanceof Base64NgError) throw caught;
    await cancelArtifactReader(reader);
    throw error("artifact-fetch", "WASM artifact stream failed");
  }
  return exactArtifactPrefix(staged, total);
}

function exactArtifactPrefix(staged, length) {
  const exact = new IntrinsicUint8Array(length);
  call(uint8Set, exact, call(uint8Subarray, staged, 0, length), 0);
  return exact;
}

function enforceArtifactSize(length) {
  if (length > MAX_ARTIFACT_BYTES) throw artifactSizeError(length);
}

function artifactSizeError(required) {
  return error("artifact-size", "WASM artifact exceeds the fixed size limit", {
    required,
    limit: MAX_ARTIFACT_BYTES,
  });
}

async function cancelArtifactReader(reader) {
  try {
    await reader.cancel();
  } catch {
    // The size failure remains authoritative even if transport cancellation fails.
  }
}

function isGenuineUint8Array(value) {
  try {
    return call(typedArrayTag, value) === "Uint8Array";
  } catch {
    return false;
  }
}

function error(code, message, details) {
  return new Base64NgError(code, message, details);
}
