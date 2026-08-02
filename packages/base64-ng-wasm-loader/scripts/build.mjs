import { copyFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../", import.meta.url));
const manifest = fileURLToPath(new URL("../wasm/Cargo.toml", import.meta.url));
const artifacts = fileURLToPath(new URL("../artifacts/", import.meta.url));

await rm(artifacts, { recursive: true, force: true });
await mkdir(artifacts, { recursive: true });

await build("scalar", [], "");
await build("simd128", ["--features", "simd"], "-C target-feature=+simd128");
await writeChecksums();

async function build(name, features, targetFeatures) {
  const targetDir = fileURLToPath(new URL(`../target-${name}/`, import.meta.url));
  const args = [
    "build",
    "--locked",
    "--manifest-path",
    manifest,
    "--target",
    "wasm32-unknown-unknown",
    "--release",
    ...features,
  ];
  const rustflags = [
    targetFeatures,
    "-C link-arg=--max-memory=8388608",
    "-C link-arg=--no-entry",
  ].filter(Boolean).join(" ");
  await run("cargo", args, {
    ...process.env,
    CARGO_TARGET_DIR: targetDir,
    RUSTFLAGS: rustflags,
  });
  await copyFile(
    `${targetDir}/wasm32-unknown-unknown/release/base64_ng_wasm_artifact.wasm`,
    `${artifacts}/base64-ng-${name}.wasm`,
  );
}

async function writeChecksums() {
  const names = ["base64-ng-scalar.wasm", "base64-ng-simd128.wasm"];
  const lines = [];
  for (const name of names) {
    const bytes = await readFile(`${artifacts}/${name}`);
    lines.push(`${createHash("sha256").update(bytes).digest("hex")}  ${name}`);
  }
  await writeFile(`${artifacts}/SHA256SUMS`, `${lines.join("\n")}\n`);
}

function run(command, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: packageRoot, env, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} failed: code=${code} signal=${signal}`));
    });
  });
}
