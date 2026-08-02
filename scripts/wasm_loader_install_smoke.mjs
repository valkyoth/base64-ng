import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";

const packageRoot = process.argv[2];
if (!packageRoot) throw new Error("usage: wasm_loader_install_smoke.mjs <package-root>");

const module = await import(pathToFileURL(`${packageRoot}/src/index.js`));
for (const artifact of ["scalar", "simd128"]) {
  const api = await module.createBase64Ng({ artifact });
  try {
    const input = new TextEncoder().encode("installed npm artifact");
    const encoded = api.encode(input, module.Codecs.URL_SAFE_NO_PAD);
    assert.deepEqual(api.decode(encoded, module.Codecs.URL_SAFE_NO_PAD), input);
    assert.equal(api.posture.selectedArtifact, artifact);
  } finally {
    api.dispose();
  }
}
