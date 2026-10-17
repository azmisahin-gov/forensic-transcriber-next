#!/usr/bin/env node
// Sets a single version across every place that must stay in sync, so a release
// can never ship a tag whose label disagrees with the binaries.
//
// usage: node scripts/bump-version.mjs 0.2.0
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2];

if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: node scripts/bump-version.mjs <x.y.z>");
  process.exit(1);
}

function patchJson(rel, mutate) {
  const path = resolve(root, rel);
  const data = JSON.parse(readFileSync(path, "utf8"));
  mutate(data);
  writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
}

// Cargo workspace version (the single source for all crates).
{
  const path = resolve(root, "Cargo.toml");
  const text = readFileSync(path, "utf8");
  const updated = text.replace(
    /(\[workspace\.package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/,
    `$1${version}$2`,
  );
  if (updated === text) {
    console.error("could not find [workspace.package] version in Cargo.toml");
    process.exit(1);
  }
  writeFileSync(path, updated);
}

// Tauri config.
patchJson("apps/desktop/src-tauri/tauri.conf.json", (j) => {
  j.version = version;
});

// Frontend package + lock.
patchJson("apps/desktop/package.json", (j) => {
  j.version = version;
});
patchJson("apps/desktop/package-lock.json", (j) => {
  j.version = version;
  if (j.packages && j.packages[""]) {
    j.packages[""].version = version;
  }
});

console.log(version);
