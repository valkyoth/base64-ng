import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";

import { Codecs, createBase64Ng } from "../src/index.js";

// 768 KiB encodes to the artifact's exact 1 MiB input ceiling, so both the
// encode and decode directions exercise the largest admitted round trip.
const input = new Uint8Array(768 * 1024);
for (let index = 0; index < input.length; index += 1) input[index] = (index * 73 + 19) & 0xff;

const scalar = await createBase64Ng({ artifact: "scalar" });
const simd = await createBase64Ng({ artifact: "simd128" });
try {
  const encoded = scalar.encode(input, Codecs.STANDARD);
  assert.deepEqual(simd.encode(input, Codecs.STANDARD), encoded);

  const scalarEncode = measure(() => scalar.encode(input, Codecs.STANDARD));
  const simdEncode = measure(() => simd.encode(input, Codecs.STANDARD));
  const scalarDecode = measure(() => scalar.decode(encoded, Codecs.STANDARD));
  const simdDecode = measure(() => simd.decode(encoded, Codecs.STANDARD));

  console.log(JSON.stringify({
    runtime: process.version,
    bytes: input.length,
    rounds: 12,
    scalarEncodeMilliseconds: scalarEncode,
    simdEncodeMilliseconds: simdEncode,
    encodeSpeedup: scalarEncode / simdEncode,
    scalarDecodeMilliseconds: scalarDecode,
    simdDecodeMilliseconds: simdDecode,
    decodeSpeedup: scalarDecode / simdDecode,
  }));

  assert.ok(simdEncode < scalarEncode, "simd128 encode must benefit this admitted Node/V8 run");
  assert.ok(simdDecode < scalarDecode, "simd128 decode must benefit this admitted Node/V8 run");
} finally {
  scalar.dispose();
  simd.dispose();
}

function measure(operation) {
  for (let index = 0; index < 4; index += 1) operation();
  const start = performance.now();
  for (let index = 0; index < 12; index += 1) operation();
  return performance.now() - start;
}
