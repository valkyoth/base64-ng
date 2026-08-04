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

  const encode = compare(
    () => scalar.encode(input, Codecs.STANDARD),
    () => simd.encode(input, Codecs.STANDARD),
  );
  const decode = compare(
    () => scalar.decode(encoded, Codecs.STANDARD),
    () => simd.decode(encoded, Codecs.STANDARD),
  );

  console.log(JSON.stringify({
    runtime: process.version,
    bytes: input.length,
    samples: 7,
    roundsPerSample: 8,
    scalarEncodeMilliseconds: encode.scalar,
    simdEncodeMilliseconds: encode.simd,
    encodeSpeedup: encode.scalar / encode.simd,
    scalarDecodeMilliseconds: decode.scalar,
    simdDecodeMilliseconds: decode.simd,
    decodeSpeedup: decode.scalar / decode.simd,
  }));

  assert.ok(encode.simd < encode.scalar, "simd128 encode median must benefit this admitted Node/V8 run");
  assert.ok(decode.simd < decode.scalar, "simd128 decode median must benefit this admitted Node/V8 run");
} finally {
  scalar.dispose();
  simd.dispose();
}

function compare(scalarOperation, simdOperation) {
  for (let index = 0; index < 8; index += 1) {
    scalarOperation();
    simdOperation();
  }
  const scalarSamples = [];
  const simdSamples = [];
  for (let sample = 0; sample < 7; sample += 1) {
    if (sample % 2 === 0) {
      scalarSamples.push(measure(scalarOperation));
      simdSamples.push(measure(simdOperation));
    } else {
      simdSamples.push(measure(simdOperation));
      scalarSamples.push(measure(scalarOperation));
    }
  }
  return { scalar: median(scalarSamples), simd: median(simdSamples) };
}

function measure(operation) {
  const start = performance.now();
  for (let index = 0; index < 8; index += 1) operation();
  return performance.now() - start;
}

function median(values) {
  const ordered = [...values].sort((left, right) => left - right);
  return ordered[Math.floor(ordered.length / 2)];
}
