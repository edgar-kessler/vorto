// Writes the third-party notices for everything compiled into vorto.exe, vorto-engine.exe and
// vorto-engine-gpu.exe: Rust crates, ONNX Runtime, whisper.cpp/ggml, the Khronos Vulkan headers
// and the web UI's runtime npm packages.
// Usage: node scripts/notices.mjs <output file>
// Needs `npm ci --prefix ui`, network access for the ONNX Runtime notices and VULKAN_SDK set to
// the Vulkan SDK the graphics card engine is built with (scripts\build.cmd sets it).
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = process.argv[2];
if (!output) throw new Error("Usage: node scripts/notices.mjs <output file>");

const LICENSE_FILE = /^(licen[cs]e|copying|notice|copyright|unlicense)([-_.].*)?$/i;
// C/C++ code vendored inside a crate, with its own license file.
const VENDORED = { "whisper-rs-sys": ["whisper.cpp/LICENSE"] };

const clean = (text) =>
  text.replace(/\r\n?/g, "\n").split("\n").map((l) => l.trimEnd()).join("\n").trim();
const licenseFiles = (dir) =>
  readdirSync(dir)
    .filter((f) => LICENSE_FILE.test(f) && statSync(join(dir, f)).isFile())
    .sort()
    .map((f) => join(dir, f));
const components = [];
const add = (name, version, license, source, files, texts = []) =>
  components.push({
    name,
    version,
    license: license || "see license text",
    source,
    texts: [...new Set([...files].map((f) => clean(readFileSync(f, "utf8"))).concat(texts))],
  });

// Rust: normal dependencies of the app and the engine for the Windows target (no build or dev deps).
const meta = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--locked", "--filter-platform", "x86_64-pc-windows-msvc"],
    { cwd: root, encoding: "utf8", maxBuffer: 1 << 30 },
  ),
);
const packages = new Map(meta.packages.map((p) => [p.id, p]));
const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
const roots = ["vorto-app", "vorto-engine"].map((name) => meta.packages.find((p) => p.name === name).id);
const linked = new Set();
const pending = [...roots];
while (pending.length) {
  for (const dep of nodes.get(pending.pop()).deps) {
    if (linked.has(dep.pkg) || !dep.dep_kinds.some((k) => k.kind === null)) continue;
    linked.add(dep.pkg);
    pending.push(dep.pkg);
  }
}
let ortVersion;
for (const id of linked) {
  if (meta.workspace_members.includes(id)) continue;
  const p = packages.get(id);
  const dir = dirname(p.manifest_path);
  const files = new Set(licenseFiles(dir));
  if (p.license_file) files.add(resolve(dir, p.license_file));
  for (const extra of VENDORED[p.name] ?? []) files.add(join(dir, extra));
  add(p.name, p.version, p.license, p.repository || `https://crates.io/crates/${p.name}`, files);
  if (p.name === "ort-sys") {
    const dist = readFileSync(join(dir, "build/download/dist.tsv"), "utf8");
    ortVersion = dist.match(/\/ms@(\d+\.\d+\.\d+)\//)?.[1];
    if (!ortVersion) throw new Error("Could not find the ONNX Runtime version in ort-sys");
  }
}

// ONNX Runtime is linked from a prebuilt static library that carries no license files.
if (ortVersion) {
  const texts = [];
  for (const file of ["LICENSE", "ThirdPartyNotices.txt"]) {
    const url = `https://raw.githubusercontent.com/microsoft/onnxruntime/v${ortVersion}/${file}`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
    texts.push(clean(await response.text()));
  }
  add("ONNX Runtime", ortVersion, "MIT", "https://github.com/microsoft/onnxruntime", [], texts);
}

// The graphics card engine compiles whisper.cpp's Vulkan backend with the Vulkan SDK's headers.
// The SDK has no license file for them, so the notice is the MIT license, one of their two, with
// the copyright line from the headers. A license file next to them would be used instead.
const vulkanSdk = process.env.VULKAN_SDK;
if (vulkanSdk) {
  const include = join(vulkanSdk, "Include", "vulkan");
  const core = join(include, "vulkan_core.h");
  const header = readFileSync(core, "utf8");
  const copyright = header.match(/Copyright [\d-]+ The Khronos Group Inc\./)?.[0];
  const license = header.match(/SPDX-License-Identifier: (.+)/)?.[1].trim();
  const api = header.match(/#define VK_HEADER_VERSION_COMPLETE VK_MAKE_API_VERSION\(0, (\d+), (\d+), VK_HEADER_VERSION\)/);
  const patch = header.match(/#define VK_HEADER_VERSION (\d+)/)?.[1];
  if (!copyright || license !== "Apache-2.0 OR MIT" || !api || !patch) {
    throw new Error(`Unexpected copyright, license or version in ${core}`);
  }
  const mit = [
    "MIT License",
    "",
    copyright,
    "",
    "Permission is hereby granted, free of charge, to any person obtaining a copy",
    'of this software and associated documentation files (the "Software"), to deal',
    "in the Software without restriction, including without limitation the rights",
    "to use, copy, modify, merge, publish, distribute, sublicense, and/or sell",
    "copies of the Software, and to permit persons to whom the Software is",
    "furnished to do so, subject to the following conditions:",
    "",
    "The above copyright notice and this permission notice shall be included in all",
    "copies or substantial portions of the Software.",
    "",
    'THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR',
    "IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,",
    "FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE",
    "AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER",
    "LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,",
    "OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE",
    "SOFTWARE.",
  ].join("\n");
  const files = licenseFiles(include);
  add(
    "Vulkan-Headers / Vulkan-Hpp",
    `${api[1]}.${api[2]}.${patch}`,
    license,
    "https://github.com/KhronosGroup/Vulkan-Headers",
    files,
    files.length ? [] : [mit],
  );
} else {
  console.warn("VULKAN_SDK isn't set, so the notices don't cover the Vulkan headers in vorto-engine-gpu.exe.");
}

// Web UI: runtime npm dependencies bundled into ui/dist.
const ui = join(root, "ui");
const findNpm = (from, name) => {
  for (let dir = from; ; dir = dirname(dir)) {
    const candidate = join(dir, "node_modules", name);
    if (existsSync(join(candidate, "package.json"))) return candidate;
    if (dir === ui) throw new Error(`${name} is not installed; run npm ci --prefix ui`);
  }
};
const npmSeen = new Set();
const npmPending = [[ui, JSON.parse(readFileSync(join(ui, "package.json"), "utf8"))]];
while (npmPending.length) {
  const [from, manifest] = npmPending.pop();
  for (const name of Object.keys(manifest.dependencies ?? {})) {
    const dir = findNpm(from, name);
    if (npmSeen.has(dir)) continue;
    npmSeen.add(dir);
    const p = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
    const source = typeof p.repository === "string" ? p.repository : p.repository?.url;
    add(p.name, p.version, p.license, source || `https://www.npmjs.com/package/${p.name}`, licenseFiles(dir));
    npmPending.push([dir, p]);
  }
}

// Group identical texts so each license is printed once.
const rule = "=".repeat(80);
const label = (c) => `${c.name} ${c.version}`;
const byLabel = (a, b) => label(a).localeCompare(label(b));
const groups = new Map();
const bare = [];
for (const c of components.sort(byLabel)) {
  if (!c.texts.length) bare.push(c);
  for (const text of c.texts) groups.set(text, [...(groups.get(text) ?? []), label(c)]);
}
const wrap = (names) =>
  names.reduce((lines, name) => {
    const last = lines.length - 1;
    if (last >= 0 && lines[last].length + name.length + 2 <= 80) lines[last] += `, ${name}`;
    else lines.push(name);
    return lines;
  }, []).join(",\n");
let out =
  "THIRD-PARTY NOTICES\n\n" +
  "Vorto is distributed under the MIT license (see LICENSE). vorto.exe, vorto-engine.exe and\n" +
  "vorto-engine-gpu.exe also contain the third-party software listed below. Each section names\n" +
  "the components, followed by the license or notice text they are distributed with.\n";
for (const [text, names] of [...groups].sort((a, b) => a[1][0].localeCompare(b[1][0]))) {
  out += `\n${rule}\n${wrap(names)}\n${rule}\n\n${text}\n`;
}
if (bare.length) {
  out += `\n${rule}\nComponents that publish no license file (their declared license applies)\n${rule}\n\n`;
  out += bare.map((c) => `${label(c)}: ${c.license} (${c.source})`).join("\n") + "\n";
}
writeFileSync(output, out);
console.log(`Wrote notices for ${components.length} components to ${output}`);
