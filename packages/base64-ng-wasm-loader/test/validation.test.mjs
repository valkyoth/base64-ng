import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { Codecs, createBase64Ng } from "../src/index.js";

async function raw(artifact) {
  const bytes = await readFile(new URL(`../artifacts/base64-ng-${artifact}.wasm`, import.meta.url));
  return (await WebAssembly.instantiate(bytes, {})).instance.exports;
}

test("production SIMD rejects every invalid byte/lane before guest output stores", async () => {
  const scalar = await raw("scalar");
  const simd = await raw("simd128");
  assert.equal(simd.base64_ng_artifact_posture(), 1);
  const input = new Uint8Array(1024).fill(65);
  const sentinel = new Uint8Array(784).fill(0xa5);
  for (let codec = 0; codec < 4; codec += 1) {
    const alphabet = codec < 2
      ? "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
      : "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    const allowed = new Set([...alphabet].map((c) => c.charCodeAt(0)));
    for (const block of [0, 496, 992]) {
      for (let lane = 0; lane < 16; lane += 1) {
        for (let byte = 0; byte < 256; byte += 1) {
          if (allowed.has(byte)) continue;
          input[block + lane] = byte;
          const results = [scalar, simd].map((exports) => {
            const memory = new Uint8Array(exports.memory.buffer);
            memory.set(input, exports.base64_ng_input_ptr());
            memory.set(sentinel, exports.base64_ng_output_ptr());
            assert.equal(exports.base64_ng_decode(input.length, codec), -1);
            assert.deepEqual(memory.slice(exports.base64_ng_output_ptr(),
              exports.base64_ng_output_ptr() + sentinel.length), sentinel);
            return [exports.base64_ng_last_error_code(), exports.base64_ng_last_error_index()];
          });
          assert.deepEqual(results[1], results[0]);
        }
        input[block + lane] = 65;
      }
    }
  }
});

test("loader SIMD boundaries, tails, capacity errors and cleanup match the scalar artifact", async () => {
  const scalar = await createBase64Ng({ artifact: "scalar" });
  const simd = await createBase64Ng({ artifact: "simd128" });
  try {
    for (const codec of Object.values(Codecs)) {
      for (const length of [0, 1, 2, 3, 11, 12, 13, 383, 384, 385, 767, 768, 769, 3071, 3072, 3073, 65536]) {
        const raw = Uint8Array.from({ length }, (_, i) => (i * 73 + 19) & 255);
        const text = scalar.encode(raw, codec);
        assert.deepEqual(simd.decode(text, codec), raw);
        const destination = new Uint8Array(length + 16).fill(0xa5);
        assert.equal(simd.decodeInto(text, destination.subarray(7, 7 + length), codec), length);
        assert.deepEqual(destination.subarray(7, 7 + length), raw);
        assert.ok(destination.subarray(0, 7).every((b) => b === 0xa5));
        assert.ok(destination.subarray(7 + length).every((b) => b === 0xa5));
        if (length > 0) {
          const short = new Uint8Array(length - 1).fill(0xa5);
          assert.throws(() => simd.decodeInto(text, short, codec));
          assert.ok(short.every((b) => b === 0xa5));
        }
        assert.deepEqual(simd.decode(text, codec), raw);
      }
    }
    for (const suffix of ["AB==", "AAB=", "A===", "AA=A", "====", "A", "AA!A"]) {
      const input = new TextEncoder().encode("A".repeat(1024) + suffix);
      const error = (api) => {
        const output = new Uint8Array(1024).fill(0xa5);
        try { api.decodeInto(input, output); }
        catch (caught) {
          assert.ok(output.every((b) => b === 0xa5));
          return [caught.code, caught.index];
        }
        assert.fail("malformed tail accepted");
      };
      assert.deepEqual(error(simd), error(scalar));
    }
  } finally {
    scalar.dispose();
    simd.dispose();
  }
});
