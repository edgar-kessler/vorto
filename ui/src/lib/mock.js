// Browser-only stand-in for the Rust backend, for designing without the app.
// Preview states: ?route=home|models|history|settings, ?onboarding=1, ?welcome=1, ?error=1,
// ?failed=<model id>, ?hook=0, ?history=off, ?update=<status>, ?hud=listening|writing|done|notice|error#hud
// with &live=, &text= and &elapsed= (seconds).
const models = [
  { id: "parakeet-v3", name: "Parakeet v3", family: "parakeet", languages: "25 European languages", downloadMb: 670, memoryMb: 900, speed: 5, accuracy: 5, hardware: "Fast on any modern processor", recommended: true, installed: true },
  { id: "whisper-turbo", name: "Whisper Turbo", family: "whisper", languages: "99 languages", downloadMb: 574, memoryMb: 1100, speed: 2, accuracy: 5, hardware: "Needs a graphics card", recommended: false, installed: true },
  { id: "whisper-small", name: "Whisper Small", family: "whisper", languages: "99 languages", downloadMb: 488, memoryMb: 850, speed: 3, accuracy: 4, hardware: "Graphics card recommended", recommended: false, installed: false },
  { id: "whisper-base", name: "Whisper Base", family: "whisper", languages: "99 languages", downloadMb: 148, memoryMb: 400, speed: 4, accuracy: 3, hardware: "Light on any processor", recommended: false, installed: false },
];
const now = new Date();
const day = (offset, time) => `${new Date(now.getTime() - offset * 86400000).toLocaleDateString("sv-SE")} ${time}`;
const hudText = { listening: "Google Chrome", writing: "Google Chrome", done: "Inserted", notice: "Too short. Hold the shortcut while you speak.", error: "Microphone unavailable" };

export function startMock({ app, level, route }) {
  const params = new URLSearchParams(location.search);
  const hudMode = params.get("hud");
  const s = {
    phase: "ready", detail: "", progress: -1, downloading: null, downloadError: null,
    recording: hudMode === "listening", recordingSince: Date.now() - Number(params.get("elapsed") ?? 0) * 1000, recordingLimit: 120,
    dictatingHere: false, live: "", dictations: 0,
    settings: { gpu: true, model: "parakeet-v3", language: "auto", microphone: "", hotkey: [163], toggle: false, live_preview: true, sounds: true, highlight: true, hud_position: "top", onboarded: !params.get("onboarding"), idle_minutes: 0, paste: true, method: "paste", append_space: false, history: params.get("history") !== "off", restore_clipboard: true, overlay: true, auto_update: true, autostart: false,
      vocabulary: ["Vorto", "Tauri", "Kessler"], dismissed: [], replacements: [{ from: "new paragraph", to: "\n\n" }],
      shortcuts: { paste_last: [0x11, 0x12, 0x56], undo_last: [], toggle_ai: [], copy_last: [] },
      ai: {
        enabled: true, timeout_secs: 20,
        providers: [{ id: "ollama", name: "Ollama", kind: "openai", base_url: "http://localhost:11434/v1", model: "qwen2.5:0.5b", allow_remote: false }],
        profiles: [
          { id: "email", name: "Email", enabled: true, prompt: "Write this as a polished, polite email body.", provider: "", model: "", apps: ["outlook.exe", "olk.exe"], titles: ["Outlook", "Gmail"] },
          { id: "prompt", name: "AI prompt", enabled: true, prompt: "Clean up a prompt for an AI assistant.", provider: "", model: "", apps: ["claude.exe"], titles: ["Claude", "ChatGPT"] },
          { id: "clean", name: "Clean up", enabled: true, prompt: "Fix grammar and remove filler words.", provider: "", model: "", apps: [], titles: [] },
        ],
      },
    },
    shortcut: ["Right Ctrl"], capturing: false, hookOk: params.get("hook") !== "0", models,
    history: [
      { text: "Hi Sarah, thanks for the quick reply. Thursday at ten works great for me, see you then.", raw: "hi sarah ähm thanks for the quick reply thursday at ten works great see you then", at: day(0, "14:32"), seconds: 6.1, app: "Google Chrome" },
      { text: "Remind me to send the updated pricing sheet to the team before the Friday review.", at: day(0, "11:05"), seconds: 4.8, app: "Slack" },
      { text: "Could you send me last month's invoice one more time? Thanks so much!", at: day(1, "18:20"), seconds: 5.2, app: "WhatsApp" },
      { text: "Refactor the settings loader so invalid values fall back to safe defaults.", at: day(3, "09:12"), seconds: 4.0, app: "Visual Studio Code" },
    ],
    wordsToday: 1284, latest: "", notice: { id: 0, text: "", kind: "info", topic: "" },
    target: { name: "Google Chrome", icon: "" },
    hud: { mode: hudMode ?? "hidden", text: params.get("text") ?? hudText[hudMode] ?? "", live: params.get("live") ?? "" },
    microphones: ["Microphone (SM950 Microphone)", "Headset (Arctis 7)"],
    dataDir: String.raw`C:\Users\you\AppData\Local\app.vorto.desktop`, version: "1.0.0", gpuBuild: true,
    micTest: false, autostart: false, appIcons: {},
    apiKeys: {}, providersLocal: [true], aiModels: {}, aiTest: { id: 0, status: "", text: "", millis: 0 },
    extraShortcuts: [["Ctrl", "Alt", "V"], [], [], []], extraOk: [true, true, true, true], suggestions: ["GitHub", "Parakeet"],
    update: { status: params.get("update") ?? "idle", version: params.get("update") ? "1.1.0" : "", progress: 0.42 },
  };
  if (params.get("welcome")) { s.models = models.map((m) => ({ ...m, installed: false })); s.phase = "setup"; }
  if (params.get("error")) { s.phase = "error"; s.detail = "The voice model stopped unexpectedly."; }
  if (params.get("failed")) s.downloadError = { id: params.get("failed"), text: "The download didn't match the published model." };
  if (params.get("route")) route.set(params.get("route"));
  const push = () => app.set(structuredClone(s));
  const inUseInstalled = () => !!s.models.find((m) => m.id === s.settings.model)?.installed;
  // A load starts quietly and ends ready, or in setup when no model is installed.
  const load = () => {
    s.phase = "starting"; s.detail = "";
    setTimeout(() => { s.phase = inUseInstalled() ? "ready" : "setup"; push(); }, 800);
  };
  let n = 0;
  const say = (text, kind = "success", topic = "") => (s.notice = { id: ++n, text, kind, topic });
  window.__vortoMock = (a) => {
    if (a.type === "startDictation") {
      // "Dictate here" records into the open window: the Dictate page shows it, not the pill.
      s.recording = true; s.recordingSince = Date.now(); s.dictatingHere = true; s.hud = { mode: "hidden", text: "", live: "" };
      const words = "So this is a quick test of the live preview while I speak to Vorto right now".split(" ");
      let i = 0;
      const t = setInterval(() => { if (!s.recording) return clearInterval(t); i = Math.min(words.length, i + 2); s.live = words.slice(0, i).join(" "); push(); }, 600);
    }
    if (a.type === "stopDictation" && s.recording) {
      s.recording = false; s.phase = "transcribing"; push();
      setTimeout(() => {
        if (!s.dictatingHere) return; // discarded meanwhile
        s.dictatingHere = false; s.phase = "ready"; s.dictations++;
        s.latest = "This is what you just said, written down.";
        if (s.settings.history) {
          s.history.unshift({ text: s.latest, at: day(0, new Date().toTimeString().slice(0, 5)), seconds: 3, app: "" });
          s.wordsToday += s.latest.split(" ").length;
        }
        push();
      }, 900);
    }
    if (a.type === "cancel") {
      s.recording = false; s.dictatingHere = false; s.live = "";
      if (s.downloading) { s.downloading = null; s.detail = ""; s.progress = -1; }
      s.phase = inUseInstalled() ? "ready" : "setup";
    }
    if (a.type === "download") {
      const model = s.models.find((m) => m.id === a.id);
      s.downloading = a.id; s.downloadError = null; s.phase = "downloading"; s.progress = 0; s.detail = "";
      const t = setInterval(() => {
        if (s.downloading !== a.id) return clearInterval(t); // cancelled
        s.progress = Math.min(1, s.progress + 0.07);
        s.detail = `Downloading ${model.name}  ·  ${Math.round(s.progress * model.downloadMb)} of ${model.downloadMb} MB`;
        if (s.progress >= 1) {
          clearInterval(t);
          s.downloading = null; s.phase = "ready"; s.progress = -1; s.detail = "";
          model.installed = true; s.settings.model = a.id;
          say(`${model.name} downloaded`, "success", "downloaded");
        }
        push();
      }, 250);
    }
    if (a.type === "retry") load();
    if (a.type === "useCpu") { s.settings.gpu = false; load(); }
    if (a.type === "useModel") s.settings.model = a.id;
    if (a.type === "saveSettings") s.settings = { ...a.settings, hotkey: s.settings.hotkey };
    if (a.type === "recordShortcut") s.capturing = true;
    if (a.type === "stopRecordingShortcut") s.capturing = false;
    if (a.type === "setShortcut") { s.capturing = false; s.shortcut = a.keys.map(String); }
    if (a.type === "clearHistory") { s.history = []; s.latest = ""; }
    if (a.type === "testMicrophone") { s.micTest = a.on; if (a.on) setTimeout(() => { s.micTest = false; push(); }, 10000); }
    if (a.type === "setAutostart") s.autostart = a.on;
    if (a.type === "checkForUpdates") { s.update = { status: "checking", version: "", progress: -1 }; setTimeout(() => { s.update.status = "latest"; push(); }, 900); }
    if (a.type === "installUpdate") { s.update.status = "installing"; }
    if (a.type === "saveSettings") s.providersLocal = s.settings.ai.providers.map((p) => ["//localhost", "//127."].some((h) => p.base_url.includes(h)));
    if (a.type === "setApiKey") s.apiKeys[a.provider] = !!a.key;
    if (a.type === "listModels") { s.aiModels[a.provider] = { status: "loading", models: [], error: "" }; setTimeout(() => { s.aiModels[a.provider] = { status: "done", models: ["gemma3:1b", "qwen2.5:0.5b", "qwen2.5:3b"], error: "" }; push(); }, 600); }
    if (a.type === "testAi") { s.aiTest = { id: s.aiTest.id + 1, status: "running", text: "", millis: 0 }; setTimeout(() => { s.aiTest = { ...s.aiTest, status: "done", text: "Können wir das Meeting morgen auf 10 Uhr verschieben? Danke!", millis: 840 }; push(); }, 900); }
    push();
  };
  push();
  setInterval(() => level.set(s.recording || s.micTest ? 0.02 + Math.random() * 0.12 : 0), 60);
}
