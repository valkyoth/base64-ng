// Directory publication rebuilds after verification. Only publish a gated tarball.
console.error(
  "Direct npm publish is disabled. From the repository root run " +
  "scripts/release_wasm_loader.sh publish-desktop (or publish for CI provenance).",
);
process.exitCode = 1;
