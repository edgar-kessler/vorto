// Development benchmark for the engine worker: download, cold load, wake-up after a
// worker restart, transcription speed and peak memory for each voice model.
// Usage: node scripts/benchmark.mjs [vorto-engine.exe] [model ...] [--cpu]
// The engine defaults to C:/vt/release (VORTO_TARGET), the clips to output/bench-en.wav and
// output/bench-de.wav: 16 kHz mono recordings of your own, which are not in the repository.
import { spawn, execFileSync } from "node:child_process";
import { createInterface } from "node:readline";
import path from "node:path";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const gpu = !args.includes("--cpu");
const rest = args.filter((a) => a !== "--cpu");
const exe = rest[0]?.endsWith(".exe") ? rest.shift() : path.join(process.env.VORTO_TARGET ?? "C:/vt", "release/vorto-engine.exe");
const models = rest.length ? rest : ["parakeet-v3", "whisper-turbo", "whisper-small", "whisper-base"];
const data = process.env.VORTO_DATA_DIR ?? path.join(process.env.LOCALAPPDATA, "app.vorto.desktop");
const clips = { en: path.join(root, "output/bench-en.wav"), de: path.join(root, "output/bench-de.wav") };

function worker() {
  const child = spawn(exe, [], { stdio: ["pipe", "pipe", "ignore"], windowsHide: true });
  const queue = [];
  let waiting = null;
  createInterface({ input: child.stdout }).on("line", (line) => {
    const event = JSON.parse(line);
    if (event.event === "progress") return;
    waiting ? (waiting(event), (waiting = null)) : queue.push(event);
  });
  const request = (command, timeout = 900_000) =>
    new Promise((resolve, reject) => {
      const started = performance.now();
      const done = (event) => {
        clearTimeout(timer);
        event.event === "error" ? reject(new Error(event.detail)) : resolve({ ...event, seconds: (performance.now() - started) / 1000 });
      };
      const timer = setTimeout(() => reject(new Error(`timeout: ${command.op}`)), timeout);
      child.stdin.write(JSON.stringify(command) + "\n");
      queue.length ? done(queue.shift()) : (waiting = done);
    });
  const peakMb = () =>
    Math.round(Number(execFileSync("powershell", ["-NoProfile", "-Command", `(Get-Process -Id ${child.pid}).PeakWorkingSet64`]).toString()) / 1e6);
  return { child, request, peakMb };
}

const results = [];
for (const model of models) {
  let w = worker();
  const row = { model, gpu };
  try {
    if (!existsSync(path.join(data, "models", model, "verified"))) {
      row.download_s = +(await w.request({ op: "download", root: data, model })).seconds.toFixed(1);
    }
    row.cold_load_s = +(await w.request({ op: "load", root: data, model, gpu })).seconds.toFixed(2);
    for (const [lang, file] of Object.entries(clips)) {
      const r = await w.request({ op: "transcribe", audio: file, language: "auto" });
      row[`${lang}_s`] = +r.seconds.toFixed(2);
      row[`${lang}_text`] = r.text;
    }
    row.peak_mb = w.peakMb();
    w.child.kill();
    w = worker();
    row.wake_s = +(await w.request({ op: "load", root: data, model, gpu })).seconds.toFixed(2);
  } catch (e) {
    row.error = e.message;
  }
  w.child.kill();
  results.push(row);
  console.log(JSON.stringify(row));
}
