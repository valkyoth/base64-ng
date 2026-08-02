import assert from "node:assert/strict";
import test from "node:test";

test("missing resizable-buffer proof intrinsics fail closed", async () => {
  const descriptor = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, "resizable");
  assert.ok(descriptor?.configurable);
  delete ArrayBuffer.prototype.resizable;
  try {
    const module = await import(`../src/index.js?missing-resizable=${Date.now()}`);
    await assert.rejects(
      module.createBase64Ng(),
      (caught) => caught.code === "runtime-intrinsics-unavailable",
    );
  } finally {
    Object.defineProperty(ArrayBuffer.prototype, "resizable", descriptor);
  }
});

test("the closed intrinsic allowlist is exercised without caller hooks", async () => {
  const counters = new Map();
  const restores = [];
  const TypedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype);
  instrumentGetter(TypedArrayPrototype, Symbol.toStringTag, "typed-tag");
  instrumentGetter(TypedArrayPrototype, "buffer", "typed-buffer");
  instrumentGetter(TypedArrayPrototype, "byteLength", "typed-byte-length");
  instrumentGetter(TypedArrayPrototype, "byteOffset", "typed-byte-offset");
  instrumentGetter(TypedArrayPrototype, "length", "typed-length");
  instrumentGetter(ArrayBuffer.prototype, "byteLength", "buffer-byte-length");
  instrumentGetter(ArrayBuffer.prototype, "resizable", "buffer-resizable");
  instrumentGetter(ArrayBuffer.prototype, "maxByteLength", "buffer-max-byte-length");
  if (typeof SharedArrayBuffer === "function") {
    instrumentGetter(SharedArrayBuffer.prototype, "byteLength", "shared-byte-length");
  }
  const originalSet = Uint8Array.prototype.set;
  Uint8Array.prototype.set = function set(...args) {
    increment("uint8-set");
    return Reflect.apply(originalSet, this, args);
  };
  restores.push(() => { Uint8Array.prototype.set = originalSet; });

  try {
    const module = await import(`../src/index.js?instrumented=${Date.now()}`);
    const api = await module.createBase64Ng({ artifact: "scalar" });
    try {
      const input = new Uint8Array([102, 111, 111]);
      const destination = new Uint8Array(4);
      assert.equal(api.encodeInto(input, destination), 4);
      assert.deepEqual(destination, new Uint8Array([90, 109, 57, 118]));
    } finally {
      api.dispose();
    }
    for (const name of [
      "typed-tag", "typed-buffer", "typed-byte-length", "typed-byte-offset",
      "typed-length", "buffer-byte-length", "buffer-resizable",
      "buffer-max-byte-length", "uint8-set",
    ]) {
      assert.ok((counters.get(name) ?? 0) > 0, `${name} was not exercised`);
    }
  } finally {
    for (const restore of restores.reverse()) restore();
  }

  function instrumentGetter(target, property, name) {
    const descriptor = Object.getOwnPropertyDescriptor(target, property);
    assert.ok(descriptor?.get && descriptor.configurable);
    Object.defineProperty(target, property, {
      ...descriptor,
      get() {
        increment(name);
        return Reflect.apply(descriptor.get, this, []);
      },
    });
    restores.push(() => Object.defineProperty(target, property, descriptor));
  }

  function increment(name) {
    counters.set(name, (counters.get(name) ?? 0) + 1);
  }
});
