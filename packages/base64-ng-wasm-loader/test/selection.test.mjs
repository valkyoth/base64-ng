import assert from "node:assert/strict";
import test from "node:test";

test("a rejected SIMD probe selects scalar without loading the SIMD artifact", async () => {
  const originalValidate = WebAssembly.validate;
  WebAssembly.validate = () => false;
  try {
    const module = await import(`../src/index.js?scalar-fallback=${Date.now()}`);
    const api = await module.createBase64Ng({
      artifact: "auto",
      simdArtifact: new Uint8Array([0]),
    });
    try {
      assert.equal(api.posture.selectedArtifact, "scalar");
      assert.equal(api.posture.selectionReason, "embedded-simd128-probe-rejected");
      assert.deepEqual(api.encode(new Uint8Array([102])), new Uint8Array([90, 103, 61, 61]));
    } finally {
      api.dispose();
    }
  } finally {
    WebAssembly.validate = originalValidate;
  }
});

test("a required SIMD deployment fails before artifact instantiation when unsupported", async () => {
  const originalValidate = WebAssembly.validate;
  WebAssembly.validate = () => false;
  try {
    const module = await import(`../src/index.js?simd-required=${Date.now()}`);
    await assert.rejects(
      module.createBase64Ng({ artifact: "simd128", simdArtifact: new Uint8Array([0]) }),
      (caught) => caught.code === "simd-unavailable",
    );
  } finally {
    WebAssembly.validate = originalValidate;
  }
});
