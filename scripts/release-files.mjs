// Collects the release files from the installer build into dist/: the installer, its update
// signature, a copy with a fixed name for "latest" download links, latest.json for the updater
// and SHA-256 checksums. Run after `scripts\build.cmd --installer` and signing the installer.
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = readFileSync(join(root, "Cargo.toml"), "utf8");
const version = cargo.match(/\[workspace\.package\][^[]*?\bversion\s*=\s*"([^"]+)"/)?.[1];
if (!version) throw new Error("No [workspace.package] version in Cargo.toml");
const ui = JSON.parse(readFileSync(join(root, "ui", "package.json"), "utf8")).version;
if (ui !== version) throw new Error(`ui/package.json has version ${ui}, Cargo.toml ${version}`);
const tag = `v${version}`;
if (process.env.GITHUB_REF_NAME && process.env.GITHUB_REF_NAME !== tag) {
  throw new Error(`The tag ${process.env.GITHUB_REF_NAME} doesn't match version ${version}`);
}
const repo = process.env.GITHUB_REPOSITORY ?? "edgar-kessler/vorto";

const nsis = join(root, "target", "release", "bundle", "nsis");
const setup = `Vorto_${version}_x64-setup.exe`;
for (const file of [setup, `${setup}.sig`]) {
  if (!existsSync(join(nsis, file))) throw new Error(`Missing ${join(nsis, file)}. Build with the signing key set.`);
}
const dist = join(root, "dist");
rmSync(dist, { recursive: true, force: true });
mkdirSync(dist);
copyFileSync(join(nsis, setup), join(dist, setup));
copyFileSync(join(nsis, `${setup}.sig`), join(dist, `${setup}.sig`));
copyFileSync(join(nsis, setup), join(dist, "Vorto-Setup.exe"));

const signature = readFileSync(join(nsis, `${setup}.sig`), "utf8").trim();
// A signature from any other key would only be a warning at build time, and every installed
// Vorto would refuse the update. Compare the minisign key ids.
const keyId = (base64) => Buffer.from(Buffer.from(base64, "base64").toString().split("\n")[1], "base64").subarray(2, 10).toString("hex");
const pubkey = JSON.parse(readFileSync(join(root, "app", "tauri.conf.json"), "utf8")).plugins.updater.pubkey;
if (keyId(signature) !== keyId(pubkey)) throw new Error("The update signature doesn't match plugins.updater.pubkey");
const url = `https://github.com/${repo}/releases/download/${tag}/${setup}`;
const latest = {
  version,
  notes: `What's new in Vorto ${version}: https://github.com/${repo}/releases/tag/${tag}`,
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
  platforms: {
    "windows-x86_64-nsis": { signature, url },
    "windows-x86_64": { signature, url },
  },
};
writeFileSync(join(dist, "latest.json"), JSON.stringify(latest, null, 2) + "\n");

const sha = (file) => createHash("sha256").update(readFileSync(join(dist, file))).digest("hex");
writeFileSync(join(dist, "SHA256SUMS.txt"), [setup, "Vorto-Setup.exe"].map((f) => `${sha(f)}  ${f}`).join("\n") + "\n");
console.log(`Release files for ${tag} in ${dist}`);
