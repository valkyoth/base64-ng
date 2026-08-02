let Codecs;
let createBase64Ng;
let status = "fail";
let detail = "browser smoke did not complete";

try {
  ({ Codecs, createBase64Ng } = await import("./package/src/index.js"));
  const codecs = [
    Codecs.STANDARD,
    Codecs.STANDARD_NO_PAD,
    Codecs.URL_SAFE,
    Codecs.URL_SAFE_NO_PAD,
  ];
  const scalar = await createBase64Ng({ artifact: "scalar" });
  const simd = await createBase64Ng({ artifact: "simd128" });
  const automatic = await createBase64Ng();
  try {
    require(automatic.posture.selectedArtifact === "simd128", "automatic SIMD selection");
    require(simd.posture.artifactPosture === "simd128", "SIMD artifact posture");
    require(scalar.posture.artifactPosture === "scalar", "scalar artifact posture");

    for (const codec of codecs) {
      for (let length = 0; length <= 200; length += 1) {
        const input = pattern(length);
        const expected = referenceEncode(input, codec);
        const scalarEncoded = scalar.encode(input, codec);
        const simdEncoded = simd.encode(input, codec);
        equalBytes(scalarEncoded, expected, "scalar encode");
        equalBytes(simdEncoded, expected, "SIMD encode");
        equalBytes(scalar.decode(expected, codec), input, "scalar decode");
        equalBytes(simd.decode(expected, codec), input, "SIMD decode");
      }
    }

    const destination = new Uint8Array(16).fill(0xa5);
    const before = new Uint8Array(destination);
    let rejected = false;
    try {
      simd.decodeInto(ascii("AA!A"), destination);
    } catch (error) {
      rejected = error.code === "invalid-byte" && error.index === 2;
    }
    require(rejected, "redacted strict error");
    equalBytes(destination, before, "transactional destination");

    const benchmarkInput = pattern(256 * 1024);
    const scalarTime = measure(() => scalar.encode(benchmarkInput));
    const simdTime = measure(() => simd.encode(benchmarkInput));
    const metrics = `scalar=${scalarTime.toFixed(3)}ms simd128=${simdTime.toFixed(3)}ms`;
    status = "pass";
    detail = `BASE64_NG_WASM_LOADER_BROWSER_PASS ${metrics}`;
  } finally {
    scalar.dispose();
    simd.dispose();
    automatic.dispose();
  }
} catch (error) {
  status = "fail";
  detail = `BASE64_NG_WASM_LOADER_BROWSER_FAIL ${error}`;
}
document.body.dataset.base64NgWasmLoaderSmoke = status;
document.getElementById("result").textContent = detail;
await reportResult(status, detail);

async function reportResult(status, detail) {
  const nonce = new URL(window.location.href).searchParams.get("nonce");
  if (nonce === null || nonce.length === 0) throw new Error("missing browser evidence nonce");
  await fetch("/__base64_ng_wasm_loader_result", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ nonce, status, detail }),
  });
}

function pattern(length) {
  const output = new Uint8Array(length);
  let value = 19;
  for (let index = 0; index < length; index += 1) {
    output[index] = value;
    value = (value + 73) & 0xff;
  }
  return output;
}

function referenceEncode(input, codec) {
  let binary = "";
  for (let index = 0; index < input.length; index += 1) binary += String.fromCharCode(input[index]);
  let encoded = btoa(binary);
  if (codec === Codecs.URL_SAFE || codec === Codecs.URL_SAFE_NO_PAD) {
    encoded = encoded.replaceAll("+", "-").replaceAll("/", "_");
  }
  if (codec === Codecs.STANDARD_NO_PAD || codec === Codecs.URL_SAFE_NO_PAD) {
    encoded = encoded.replace(/=+$/u, "");
  }
  return ascii(encoded);
}

function ascii(text) {
  const output = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index += 1) output[index] = text.charCodeAt(index);
  return output;
}

function equalBytes(left, right, label) {
  require(left.length === right.length, `${label} length`);
  for (let index = 0; index < left.length; index += 1) {
    require(left[index] === right[index], `${label} byte ${index}`);
  }
}

function require(condition, label) {
  if (!condition) throw new Error(label);
}

function measure(operation) {
  operation();
  operation();
  const start = performance.now();
  for (let index = 0; index < 8; index += 1) operation();
  return performance.now() - start;
}
