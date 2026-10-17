#!/usr/bin/env node
// Fails if any version field disagrees, so CI blocks a broken release before a
// tag can be created.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const cargo = readFileSync(resolve(root, "Cargo.toml"), "utf8");
const cargoMatch = cargo.match(/\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"/);
const tauri = JSON.parse(
  readFileSync(resolve(root, "apps/desktop/src-tauri/tauri.conf.json"), "utf8"),
).version;
const pkg = JSON.parse(readFileSync(resolve(root, "apps/desktop/package.json"), "utf8")).version;
const lock = JSON.parse(
  readFileSync(resolve(root, "apps/desktop/package-lock.json"), "utf8"),
);

const versions = {
  "Cargo.toml": cargoMatch ? cargoMatch[1] : "(missing)",
  "tauri.conf.json": tauri,
  "package.json": pkg,
  "package-lock.json": lock.version,
  "package-lock.json[packages['']]": lock.packages && lock.packages[""] ? lock.packages[""].version : "(missing)",
};

const unique = new Set(Object.values(versions));
for (const [file, value] of Object.entries(versions)) {
  console.log(`${file}: ${value}`);
}
if (unique.size !== 1) {
  console.error("Version mismatch across files.");
  process.exit(1);
}
console.log(`All version fields agree: ${[...unique][0]}`);
