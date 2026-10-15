#!/usr/bin/env node
// Build RouDex's installers for one target (spec 005, D3).
//
//   node scripts/bundle.mjs [--target <triple>] [--bundles nsis|app,dmg|deb,appimage]
//
// 1. Builds router-cli in release for the target and puts it where Tauri
//    expects an external binary: src-tauri/binaries/router-cli-<triple>[.exe].
//    For universal-apple-darwin both Mac architectures are built and joined
//    with lipo.
// 2. Runs `tauri build` with src-tauri/tauri.bundle.json merged in. The
//    sidecar lives only in that file so plain `cargo build` and `cargo test`
//    do not need it.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const option = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};
const run = (command, commandArgs, options = {}) =>
  execFileSync(command, commandArgs, { cwd: root, stdio: "inherit", ...options });

const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(/^host: (\S+)$/m)[1];
const target = option("--target") ?? host;
const bundles = option("--bundles");
const binaries = join(root, "src-tauri", "binaries");
mkdirSync(binaries, { recursive: true });

function buildRouterCli(triple) {
  const cargoArgs = ["build", "--release", "--locked", "-p", "router-cli"];
  if (triple !== host) cargoArgs.push("--target", triple);
  run("cargo", cargoArgs);
  const exe = triple.includes("windows") ? ".exe" : "";
  return join(root, "target", ...(triple === host ? [] : [triple]), "release", `router-cli${exe}`);
}

if (target === "universal-apple-darwin") {
  const parts = ["aarch64-apple-darwin", "x86_64-apple-darwin"].map((triple) => {
    // Building for the host under its own triple keeps both outputs apart.
    const cargoArgs = ["build", "--release", "--locked", "-p", "router-cli", "--target", triple];
    run("cargo", cargoArgs);
    return join(root, "target", triple, "release", "router-cli");
  });
  run("lipo", ["-create", "-output", join(binaries, "router-cli-universal-apple-darwin"), ...parts]);
} else {
  const exe = target.includes("windows") ? ".exe" : "";
  copyFileSync(buildRouterCli(target), join(binaries, `router-cli-${target}${exe}`));
}

const tauri = join(root, "ui", "node_modules", "@tauri-apps", "cli", "tauri.js");
const tauriArgs = [tauri, "build", "--config", join("src-tauri", "tauri.bundle.json")];
if (target !== host) tauriArgs.push("--target", target);
if (bundles) tauriArgs.push("--bundles", bundles);
run(process.execPath, tauriArgs);
