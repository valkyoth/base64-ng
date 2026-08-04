import { copyFile, mkdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../", import.meta.url));
const repositoryRoot = await realpath(fileURLToPath(new URL("../../../", import.meta.url)));
const manifest = fileURLToPath(new URL("../wasm/Cargo.toml", import.meta.url));
const artifacts = fileURLToPath(new URL("../artifacts/", import.meta.url));

await rm(artifacts, { recursive: true, force: true });
await mkdir(artifacts, { recursive: true });

await build("scalar", [], "");
await build("simd128", ["--features", "simd"], "target-feature=+simd128");
await writeChecksums();
await writeProvenance();

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
    ...(targetFeatures ? ["-C", targetFeatures] : []),
    `--remap-path-prefix=${repositoryRoot}=.`,
    "-C",
    "link-arg=--max-memory=8388608",
    "-C",
    "link-arg=--no-entry",
  ];
  const env = {
    ...process.env,
    CARGO_TARGET_DIR: targetDir,
    CARGO_ENCODED_RUSTFLAGS: rustflags.join("\x1f"),
  };
  delete env.RUSTFLAGS;
  await run("cargo", args, env);
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

async function writeProvenance() {
  const packageMetadata = JSON.parse(
    await readFile(`${packageRoot}/package.json`, "utf8"),
  );
  const sourceCommit = process.env.BASE64_NG_SOURCE_COMMIT
    ?? await capture("git", ["rev-parse", "HEAD"], repositoryRoot);
  if (!/^[0-9a-f]{40}$/u.test(sourceCommit)) {
    throw new Error("BASE64_NG_SOURCE_COMMIT must be a full lowercase Git commit");
  }

  const artifactsByName = {};
  for (const name of ["base64-ng-scalar.wasm", "base64-ng-simd128.wasm"]) {
    const bytes = await readFile(`${artifacts}/${name}`);
    artifactsByName[name] = createHash("sha256").update(bytes).digest("hex");
  }
  const provenance = {
    schema: "base64-ng-wasm-provenance-v1",
    package: packageMetadata.name,
    version: packageMetadata.version,
    sourceCommit,
    artifacts: artifactsByName,
  };
  await writeFile(
    `${artifacts}/PROVENANCE.json`,
    `${JSON.stringify(provenance, null, 2)}\n`,
  );
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

function capture(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: ["ignore", "pipe", "inherit"] });
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { output += chunk; });
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) resolve(output.trim());
      else reject(new Error(`${command} failed: code=${code} signal=${signal}`));
    });
  });
}
