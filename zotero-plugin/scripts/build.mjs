/**
 * Build script: compile TypeScript → bundle JS → package .xpi
 */

import * as esbuild from "esbuild";
import { readFileSync, writeFileSync, cpSync, mkdirSync, rmSync, existsSync } from "fs";
import { execSync } from "child_process";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const buildDir = join(root, "build");
const addonDir = join(buildDir, "addon");

// Clean
if (existsSync(buildDir)) {
  rmSync(buildDir, { recursive: true });
}
mkdirSync(buildDir, { recursive: true });

// Copy addon/ to build/addon/
cpSync(join(root, "addon"), addonDir, { recursive: true });

// Bundle TypeScript
await esbuild.build({
  entryPoints: [join(root, "src/index.ts")],
  bundle: true,
  outfile: join(addonDir, "content/zoterorag.js"),
  format: "iife",
  globalName: "ZoteroRAG",
  target: "firefox115",
  platform: "browser",
  external: [],
  define: {
    "process.env.NODE_ENV": '"production"',
  },
  // Export all named exports as properties of the global
  footer: {
    js: `
// Expose module exports as global ZoteroRAG
if (typeof ZoteroRAG !== "undefined" && ZoteroRAG.default) {
  Object.assign(ZoteroRAG, ZoteroRAG.default);
}
`,
  },
});

console.log("Bundled src/index.ts → content/zoterorag.js");

// Read package.json for version
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

// Update manifest.json version
const manifestPath = join(addonDir, "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
manifest.version = pkg.version;
writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

// Package as .xpi (just a zip)
const xpiName = `zotero-rag-library-${pkg.version}.xpi`;
execSync(`cd "${addonDir}" && zip -r "${join(buildDir, xpiName)}" .`, {
  stdio: "inherit",
});

console.log(`Built: build/${xpiName}`);
