// The voices of the website's show. Reads scripts/voice-lines.json and writes one MP3 per line to
// site/audio. Run scripts/voice-times.py afterwards, for the moment each word is said.
//
//   OPENROUTER_API_KEY=... node scripts/voice.mjs [id ...]
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const MODEL = "elevenlabs/eleven-v4";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const key = process.env.OPENROUTER_API_KEY;
if (!key) {
  console.error("Set OPENROUTER_API_KEY first.");
  process.exit(1);
}
const only = process.argv.slice(2);
const lines = JSON.parse(readFileSync(join(root, "scripts", "voice-lines.json"), "utf8"));
mkdirSync(join(root, "site", "audio"), { recursive: true });

for (const line of lines.filter((l) => !only.length || only.includes(l.id))) {
  const reply = await fetch("https://openrouter.ai/api/v1/audio/speech", {
    method: "POST",
    headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
    body: JSON.stringify({ model: MODEL, input: line.say, voice: line.voice, response_format: "mp3" }),
  });
  if (!reply.ok) {
    console.error(`${line.id}: ${reply.status} ${(await reply.text()).slice(0, 300)}`);
    process.exitCode = 1;
    continue;
  }
  const audio = Buffer.from(await reply.arrayBuffer());
  writeFileSync(join(root, "site", "audio", `${line.id}.mp3`), audio);
  console.log(`${line.id}: ${Math.round(audio.length / 1024)} KB`);
}
