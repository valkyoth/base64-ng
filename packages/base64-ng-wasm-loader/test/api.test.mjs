import assert from "node:assert/strict";
import test from "node:test";

import { Base64NgError, Codecs, createBase64Ng } from "../src/index.js";

const codecs = [
  Codecs.STANDARD,
  Codecs.STANDARD_NO_PAD,
  Codecs.URL_SAFE,
  Codecs.URL_SAFE_NO_PAD,
];

for (const artifact of ["scalar", "simd128"]) {
  test(`${artifact}: all codecs match independent canonical encoding`, async () => {
    const api = await createBase64Ng({ artifact });
    try {
      for (const codec of codecs) {
        for (let length = 0; length <= 513; length += 1) {
          const input = pattern(length, 19);
          const expected = referenceEncode(input, codec);
          const encoded = api.encode(input, codec);
          assert.deepEqual(encoded, expected);
          assert.equal(api.encodedLength(input, codec), expected.length);
          assert.equal(api.decodedLength(encoded, codec), input.length);
          assert.deepEqual(api.decode(encoded, codec), input);

          const encodedInto = new Uint8Array(expected.length + 7).fill(0xa5);
          assert.equal(api.encodeInto(input, encodedInto, codec), expected.length);
          assert.deepEqual(encodedInto.slice(0, expected.length), expected);
          assert.deepEqual(encodedInto.slice(expected.length), new Uint8Array(7).fill(0xa5));

          const decodedInto = new Uint8Array(input.length + 7).fill(0x5a);
          assert.equal(api.decodeInto(encoded, decodedInto, codec), input.length);
          assert.deepEqual(decodedInto.slice(0, input.length), input);
          assert.deepEqual(decodedInto.slice(input.length), new Uint8Array(7).fill(0x5a));
        }
      }
    } finally {
      api.dispose();
    }
  });

  test(`${artifact}: strict errors are stable, redacted, and transactional`, async () => {
    const api = await createBase64Ng({ artifact });
    try {
      const cases = [
        [bytes("A"), "invalid-length", undefined],
        [bytes("AA!A"), "invalid-byte", 2],
        [bytes("AB=="), "invalid-padding", 1],
      ];
      for (const [input, code, index] of cases) {
        const destination = new Uint8Array(32).fill(0x73);
        const before = destination.slice();
        assert.throws(
          () => api.decodeInto(input, destination),
          (caught) => {
            assert.ok(caught instanceof Base64NgError);
            assert.equal(caught.code, code);
            assert.equal(caught.index, index);
            assert.ok(!JSON.stringify(caught).includes("0x"));
            return true;
          },
        );
        assert.deepEqual(destination, before);
      }

      const destination = new Uint8Array(2).fill(0x41);
      assert.throws(
        () => api.decodeInto(bytes("Zm9v"), destination),
        (caught) => caught.code === "output-capacity" && caught.required === 3,
      );
      assert.deepEqual(destination, new Uint8Array([0x41, 0x41]));
    } finally {
      api.dispose();
    }
  });

  test(`${artifact}: forgiving decode follows the byte-only WHATWG surface`, async () => {
    const api = await createBase64Ng({ artifact });
    try {
      assert.deepEqual(api.decodeForgiving(bytes(" Z\tg\n=\f=\r ")), bytes("f"));
      assert.deepEqual(api.decodeForgiving(bytes("Zh")), bytes("f"));
      for (const malformed of ["Z", "Zg=", "Zg===", "Z=g=", "AA-A"]) {
        assert.throws(
          () => api.decodeForgiving(bytes(malformed)),
          (caught) => caught.code === "invalid-input",
        );
      }
    } finally {
      api.dispose();
    }
  });
}

test("automatic selection and explicit deployment contracts are reported", async () => {
  const automatic = await createBase64Ng();
  try {
    assert.equal(automatic.posture.capabilityEvidence, "embedded-probe-validated");
    assert.equal(automatic.posture.selectedArtifact, "simd128");
    assert.equal(automatic.posture.artifactPosture, "simd128");
    assert.equal(automatic.posture.secretApi, false);
  } finally {
    automatic.dispose();
  }

  const scalar = await createBase64Ng({ artifact: "scalar" });
  try {
    assert.equal(scalar.posture.selectedArtifact, "scalar");
    assert.equal(scalar.posture.selectionReason, "deployment-forced-scalar");
  } finally {
    scalar.dispose();
  }
});

test("limits, WASM32 integer bounds, and disposal fail closed", async () => {
  for (const value of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1, 0x1_0000_0000]) {
    await assert.rejects(
      createBase64Ng({ maxInputLength: value }),
      (caught) => caught.code === "invalid-limit",
    );
  }
  await assert.rejects(
    createBase64Ng({ maximumMemoryPages: 129 }),
    (caught) => caught.code === "memory-limit" && caught.limit === 128,
  );

  const api = await createBase64Ng({ maxInputLength: 4, maxOutputLength: 8 });
  assert.throws(
    () => api.encode(new Uint8Array(5)),
    (caught) => caught.code === "input-limit" && caught.limit === 4,
  );
  api.dispose();
  api.dispose();
  assert.throws(() => api.encode(new Uint8Array()), (caught) => caught.code === "disposed");
});

function bytes(text) {
  return new TextEncoder().encode(text);
}

function pattern(length, seed) {
  const output = new Uint8Array(length);
  let value = seed;
  for (let index = 0; index < output.length; index += 1) {
    output[index] = value;
    value = (value + 73) & 0xff;
  }
  return output;
}

function referenceEncode(input, codec) {
  let encoded = Buffer.from(input).toString("base64");
  if (codec === Codecs.URL_SAFE || codec === Codecs.URL_SAFE_NO_PAD) {
    encoded = encoded.replaceAll("+", "-").replaceAll("/", "_");
  }
  if (codec === Codecs.STANDARD_NO_PAD || codec === Codecs.URL_SAFE_NO_PAD) {
    encoded = encoded.replace(/=+$/u, "");
  }
  return new Uint8Array(Buffer.from(encoded, "ascii"));
}
