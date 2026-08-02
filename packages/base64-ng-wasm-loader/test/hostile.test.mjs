import assert from "node:assert/strict";
import vm from "node:vm";
import test from "node:test";
import { Worker } from "node:worker_threads";

import { Codecs, createBase64Ng } from "../src/index.js";

test("cross-realm genuine Uint8Array values are accepted", async () => {
  const input = vm.runInNewContext("new Uint8Array([102, 111, 111])");
  const destination = vm.runInNewContext("new Uint8Array(4)");
  const api = await createBase64Ng();
  try {
    assert.deepEqual(api.encode(input), new Uint8Array([90, 109, 57, 118]));
    assert.equal(api.encodeInto(input, destination), 4);
    assert.deepEqual(new Uint8Array(destination), new Uint8Array([90, 109, 57, 118]));
  } finally {
    api.dispose();
  }
});

test("hostile subclasses cannot invoke constructors, iterators, species, or helpers", async () => {
  let invoked = 0;
  class HostileBytes extends Uint8Array {
    static get [Symbol.species]() { invoked += 1; throw new Error("species"); }
    [Symbol.iterator]() { invoked += 1; throw new Error("iterator"); }
    set() { invoked += 1; throw new Error("set"); }
    slice() { invoked += 1; throw new Error("slice"); }
    subarray() { invoked += 1; throw new Error("subarray"); }
  }
  const input = new HostileBytes([102, 111, 111]);
  const destination = new HostileBytes(4);
  Object.defineProperty(input, "constructor", {
    get() { invoked += 1; throw new Error("constructor"); },
  });
  Object.defineProperty(destination, "constructor", {
    get() { invoked += 1; throw new Error("constructor"); },
  });
  const api = await createBase64Ng();
  try {
    assert.deepEqual(api.encode(input), new Uint8Array([90, 109, 57, 118]));
    assert.equal(api.encodeInto(input, destination, Codecs.STANDARD), 4);
    const observed = new Uint8Array(4);
    Uint8Array.prototype.set.call(observed, destination);
    assert.deepEqual(observed, new Uint8Array([90, 109, 57, 118]));
    assert.equal(invoked, 0);
  } finally {
    api.dispose();
  }
});

test("proxies and property-spoofed objects are rejected without invoking traps", async () => {
  let traps = 0;
  const proxy = new Proxy(new Uint8Array([1, 2, 3]), {
    get() { traps += 1; throw new Error("get trap"); },
    getPrototypeOf() { traps += 1; throw new Error("prototype trap"); },
  });
  const spoof = {
    0: 1,
    length: 1,
    byteLength: 1,
    byteOffset: 0,
    buffer: new ArrayBuffer(1),
    [Symbol.toStringTag]: "Uint8Array",
  };
  const api = await createBase64Ng();
  try {
    for (const value of [proxy, spoof, [1, 2, 3], new Uint16Array([1])]) {
      assert.throws(() => api.encode(value), (caught) => caught.code === "invalid-byte-view");
    }
    assert.equal(traps, 0);
  } finally {
    api.dispose();
  }
});

test("shared, resizable, detached, and overlapping storage is rejected", async (context) => {
  const api = await createBase64Ng();
  try {
    if (typeof SharedArrayBuffer === "function") {
      const shared = new Uint8Array(new SharedArrayBuffer(8));
      assert.throws(() => api.encode(shared), (caught) => caught.code === "shared-buffer");
      assert.throws(
        () => api.encodeInto(new Uint8Array([1]), shared),
        (caught) => caught.code === "shared-buffer",
      );
    }

    if (typeof ArrayBuffer.prototype.resize === "function") {
      const resizable = new Uint8Array(new ArrayBuffer(8, { maxByteLength: 16 }));
      assert.throws(() => api.encode(resizable), (caught) => caught.code === "resizable-buffer");
    } else {
      context.diagnostic("Resizable ArrayBuffer is unavailable in this Node runtime");
    }

    const detachedBuffer = new ArrayBuffer(8);
    const detached = new Uint8Array(detachedBuffer);
    structuredClone(detachedBuffer, { transfer: [detachedBuffer] });
    assert.throws(() => api.encode(detached), (caught) => caught.code === "detached-buffer");
    assert.throws(
      () => api.encodeInto(new Uint8Array([1]), detached),
      (caught) => caught.code === "detached-buffer",
    );

    const storage = new Uint8Array(16);
    const input = new Uint8Array(storage.buffer, 0, 6);
    const overlapping = new Uint8Array(storage.buffer, 4, 8);
    assert.throws(
      () => api.encodeInto(input, overlapping),
      (caught) => caught.code === "overlapping-views",
    );
    const adjacent = new Uint8Array(storage.buffer, 6, 8);
    assert.equal(api.encodeInto(input, adjacent), 8);
  } finally {
    api.dispose();
  }
});

test("every public byte operation applies the same brand and backing gate", async () => {
  const api = await createBase64Ng();
  const invalid = { [Symbol.toStringTag]: "Uint8Array", length: 0 };
  try {
    const operations = [
      () => api.encodedLength(invalid),
      () => api.decodedLength(invalid),
      () => api.encode(invalid),
      () => api.decode(invalid),
      () => api.encodeInto(invalid, new Uint8Array(4)),
      () => api.decodeInto(invalid, new Uint8Array(4)),
      () => api.decodeForgiving(invalid),
    ];
    for (const operation of operations) {
      assert.throws(operation, (caught) => caught.code === "invalid-byte-view");
    }
  } finally {
    api.dispose();
  }
});

test("codec selection does not coerce caller-controlled objects", async () => {
  let invoked = 0;
  const codec = {
    toString() { invoked += 1; throw new Error("toString"); },
    valueOf() { invoked += 1; throw new Error("valueOf"); },
    [Symbol.toPrimitive]() { invoked += 1; throw new Error("toPrimitive"); },
  };
  const api = await createBase64Ng();
  try {
    assert.throws(
      () => api.encode(new Uint8Array(), codec),
      (caught) => caught.code === "invalid-codec",
    );
    assert.throws(
      () => api.encode(new Uint8Array(), "toString"),
      (caught) => caught.code === "invalid-codec",
    );
    assert.equal(invoked, 0);
  } finally {
    api.dispose();
  }
});

test("worker-mutated shared input is rejected by every byte-reading API", async (context) => {
  if (typeof SharedArrayBuffer !== "function") {
    context.skip("SharedArrayBuffer is unavailable");
    return;
  }
  const storage = new SharedArrayBuffer(64);
  const input = new Uint8Array(storage);
  const worker = new Worker(
    "const { workerData } = require('node:worker_threads');"
      + " setInterval(() => Atomics.add(new Uint8Array(workerData), 0, 1), 0)",
    { eval: true, workerData: storage },
  );
  const api = await createBase64Ng();
  try {
    const operations = [
      () => api.encodedLength(input),
      () => api.decodedLength(input),
      () => api.encode(input),
      () => api.decode(input),
      () => api.encodeInto(input, new Uint8Array(128)),
      () => api.decodeInto(input, new Uint8Array(128)),
      () => api.decodeForgiving(input),
    ];
    for (const operation of operations) {
      assert.throws(operation, (caught) => caught.code === "shared-buffer");
    }
  } finally {
    api.dispose();
    await worker.terminate();
  }
});
