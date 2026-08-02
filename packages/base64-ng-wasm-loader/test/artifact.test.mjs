import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

for (const artifact of ["scalar", "simd128"]) {
  test(`${artifact}: artifact memory ceiling and current-buffer clearing are enforced`, async () => {
    const bytes = await readFile(new URL(`../artifacts/base64-ng-${artifact}.wasm`, import.meta.url));
    const { instance } = await WebAssembly.instantiate(bytes, {});
    const exports = instance.exports;
    const inputPointer = exports.base64_ng_input_ptr();
    const outputPointer = exports.base64_ng_output_ptr();
    const inputCapacity = exports.base64_ng_input_capacity();
    const outputCapacity = exports.base64_ng_output_capacity();
    const memory = new Uint8Array(exports.memory.buffer);
    memory.fill(0xa5, inputPointer, inputPointer + inputCapacity);
    memory.fill(0x5a, outputPointer, outputPointer + outputCapacity);
    exports.base64_ng_clear();
    assert.ok(memory.subarray(inputPointer, inputPointer + inputCapacity).every((byte) => byte === 0));
    assert.ok(memory.subarray(outputPointer, outputPointer + outputCapacity).every((byte) => byte === 0));

    const currentPages = exports.memory.buffer.byteLength / 65536;
    if (currentPages < 128) exports.memory.grow(128 - currentPages);
    assert.equal(exports.memory.buffer.byteLength / 65536, 128);
    assert.throws(() => exports.memory.grow(1), RangeError);
  });
}
