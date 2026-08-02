import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, open, readFile, rm } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";

import { createBase64Ng } from "../src/index.js";

for (const artifact of ["scalar", "simd128"]) {
  test(`${artifact}: artifact memory ceiling and tracked clearing are enforced`, async () => {
    const bytes = await readFile(new URL(`../artifacts/base64-ng-${artifact}.wasm`, import.meta.url));
    const { instance } = await WebAssembly.instantiate(bytes, {});
    const exports = instance.exports;
    const inputPointer = exports.base64_ng_input_ptr();
    const outputPointer = exports.base64_ng_output_ptr();
    const inputCapacity = exports.base64_ng_input_capacity();
    const outputCapacity = exports.base64_ng_output_capacity();
    const memory = new Uint8Array(exports.memory.buffer);

    memory.fill(0xa5, inputPointer, inputPointer + 16);
    memory.fill(0x5a, outputPointer, outputPointer + 16);
    exports.base64_ng_clear_used(3, 5);
    assert.deepEqual(
      memory.slice(inputPointer, inputPointer + 16),
      new Uint8Array([0, 0, 0, ...new Array(13).fill(0xa5)]),
    );
    assert.deepEqual(
      memory.slice(outputPointer, outputPointer + 16),
      new Uint8Array([0, 0, 0, 0, 0, ...new Array(11).fill(0x5a)]),
    );

    exports.base64_ng_clear_used(0, 0);
    assert.equal(memory[inputPointer + 3], 0xa5);
    assert.equal(memory[outputPointer + 5], 0x5a);

    const input = new TextEncoder().encode("tracked actual output");
    memory.set(input, inputPointer);
    const written = exports.base64_ng_encode(input.length, 0);
    assert.ok(written > 0);
    assert.ok(memory.subarray(outputPointer, outputPointer + written).some((byte) => byte !== 0));
    exports.base64_ng_clear_used(input.length, 0);
    assert.ok(
      memory.subarray(outputPointer, outputPointer + written).every((byte) => byte === 0),
      "Rust-owned actual output tracking must override a stale zero cleanup bound",
    );

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

test("custom artifacts require and enforce an explicit SHA-256 digest", async () => {
  const bytes = await readFile(new URL("../artifacts/base64-ng-scalar.wasm", import.meta.url));
  const expected = createHash("sha256").update(bytes).digest("hex");

  await assert.rejects(
    createBase64Ng({ artifact: "scalar", scalarArtifact: bytes }),
    (caught) => caught.code === "artifact-integrity-policy",
  );

  const tampered = new Uint8Array(bytes);
  tampered[tampered.length - 1] ^= 1;
  await assert.rejects(
    createBase64Ng({
      artifact: "scalar",
      scalarArtifact: tampered,
      scalarArtifactSha256: expected,
    }),
    (caught) => caught.code === "artifact-integrity",
  );

  const api = await createBase64Ng({
    artifact: "scalar",
    scalarArtifact: bytes,
    scalarArtifactSha256: expected,
  });
  try {
    assert.equal(api.posture.artifactSha256, expected);
  } finally {
    api.dispose();
  }
});

test("artifact acquisition rejects arrays, files, and streams above the fixed ceiling", async () => {
  const limit = 1024 * 1024;
  const digest = "0".repeat(64);
  const oversized = new Uint8Array(limit + 1);
  await assert.rejects(
    createBase64Ng({
      artifact: "scalar",
      scalarArtifact: oversized,
      scalarArtifactSha256: digest,
    }),
    (caught) => caught.code === "artifact-size"
      && caught.required === limit + 1
      && caught.limit === limit,
  );

  const directory = await mkdtemp(join(tmpdir(), "base64-ng-artifact-limit-"));
  const artifactPath = join(directory, "oversized.wasm");
  const file = await open(artifactPath, "w");
  await file.truncate(limit + 1);
  await file.close();
  try {
    await assert.rejects(
      createBase64Ng({
        artifact: "scalar",
        scalarArtifact: pathToFileURL(artifactPath),
        scalarArtifactSha256: digest,
      }),
      (caught) => caught.code === "artifact-size" && caught.limit === limit,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }

  const server = createServer((_request, response) => {
    response.writeHead(200, { "content-type": "application/wasm" });
    const chunk = new Uint8Array(64 * 1024);
    for (let index = 0; index < 17; index += 1) response.write(chunk);
    response.end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  try {
    await assert.rejects(
      createBase64Ng({
        artifact: "scalar",
        scalarArtifact: new URL(`http://127.0.0.1:${address.port}/oversized.wasm`),
        scalarArtifactSha256: digest,
      }),
      (caught) => caught.code === "artifact-size" && caught.limit === limit,
    );
  } finally {
    await new Promise((resolve, reject) => server.close((failure) => {
      if (failure) reject(failure);
      else resolve();
    }));
  }
});
