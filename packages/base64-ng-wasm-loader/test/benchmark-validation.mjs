// Trusted local baseline package only. This executes its loader JavaScript.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { performance } from "node:perf_hooks";

if (process.argv.length !== 3) throw new Error("usage: node benchmark-validation.mjs BASELINE_PACKAGE");
const roots = [resolve(process.argv[2]), fileURLToPath(new URL("../", import.meta.url))];
const loaders = await Promise.all(roots.map((root) => import(pathToFileURL(`${root}/src/index.js`))));
const rows = [];
const hashes = [];
for (const artifact of ["scalar", "simd128"]) {
  const bytes = await Promise.all(roots.map((root) => readFile(`${root}/artifacts/base64-ng-${artifact}.wasm`)));
  hashes.push({ artifact, sha256: bytes.map((b) => createHash("sha256").update(b).digest("hex")) });
  const apis = await Promise.all(loaders.map((loader) => loader.createBase64Ng({ artifact })));
  const guests = await Promise.all(bytes.map(async (b) => (await WebAssembly.instantiate(b, {})).instance.exports));
  try {
    for (let codec = 0; codec < 4; codec += 1) {
      const key = ["STANDARD", "STANDARD_NO_PAD", "URL_SAFE", "URL_SAFE_NO_PAD"][codec];
      const policies = loaders.map((loader) => loader.Codecs[key]);
      for (const size of [0, 3, 32, 384, 1024, 4096, 65536, 786432]) {
        const input = Uint8Array.from({ length: size }, (_, i) => (i * 73 + 19) & 255);
        let text = Buffer.from(input).toString("base64");
        if (codec >= 2) text = text.replaceAll("+", "-").replaceAll("/", "_");
        if (codec % 2) text = text.replace(/=+$/u, "");
        const encoded = new Uint8Array(Buffer.from(text, "ascii"));
        const outputs = [new Uint8Array(size), new Uint8Array(size)];
        for (const guest of guests) new Uint8Array(guest.memory.buffer).set(encoded, guest.base64_ng_input_ptr());
        const rounds = size < 4096 ? 128 : size < 65536 ? 64 : 8;
        for (const surface of ["guest-decode", "loader-decode", "loader-decodeInto"]) {
          const call = (variant) => {
            if (surface === "guest-decode") return guests[variant].base64_ng_decode(encoded.length, codec);
            if (surface === "loader-decode") return apis[variant].decode(encoded, policies[variant]).length;
            return apis[variant].decodeInto(encoded, outputs[variant], policies[variant]);
          };
          const verify = (variant) => {
            assert.equal(call(variant), size);
            if (surface === "guest-decode") {
              const guest = guests[variant];
              assert.deepEqual(new Uint8Array(guest.memory.buffer).slice(guest.base64_ng_output_ptr(),
                guest.base64_ng_output_ptr() + size), input);
            } else if (surface === "loader-decode") {
              assert.deepEqual(apis[variant].decode(encoded, policies[variant]), input);
            } else assert.deepEqual(outputs[variant], input);
          };
          for (let warm = 0; warm < 16; warm += 1) { call(0); call(1); }
          verify(0); verify(1);
          for (let sample = 0; sample < 7; sample += 1) {
            for (const variant of [sample % 2, 1 - sample % 2]) {
              let total = 0;
              const start = performance.now();
              for (let i = 0; i < rounds; i += 1) total += call(variant);
              const elapsed = performance.now() - start;
              assert.equal(total, rounds * size);
              assert.ok(elapsed > 0);
              rows.push({ artifact, codec, size, surface, sample, variant, rounds, milliseconds: elapsed });
            }
          }
          verify(0); verify(1);
        }
      }
    }
  } finally {
    for (const api of apis) api.dispose();
    for (const guest of guests) guest.base64_ng_clear();
  }
}
console.log(JSON.stringify({ runtime: process.version, v8: process.versions.v8,
  platform: process.platform, arch: process.arch, hashes, samples: 7,
  scope: "warm public artifact comparison; guest includes validation/decode; loader includes copying/cleanup",
  rows }, null, 2));
