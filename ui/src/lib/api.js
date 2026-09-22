import { writable, get } from "svelte/store";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
export const app = writable(null);
export const level = writable(0);
export const route = writable("home");
/** The open onboarding step, or null when onboarding is closed. */
export const onboardingStep = writable(null);

let invoke = null;

export async function connect() {
  if (!isTauri) {
    const { startMock } = await import("./mock.js");
    startMock({ app, level, route });
    return;
  }
  ({ invoke } = await import("@tauri-apps/api/core"));
  const { listen } = await import("@tauri-apps/api/event");
  // The pill gets a small state of its own. Event names differ per window because every
  // listener receives its event name, whichever window it was sent to.
  const hud = location.hash === "#hud";
  // Listen first so no update between the query and the subscription is lost.
  await listen(hud ? "hud-state" : "state", (e) => app.set(e.payload));
  await listen(hud ? "hud-level" : "level", (e) => level.set(e.payload));
  if (!hud) await listen("navigate", (e) => route.set(e.payload));
  const initial = await invoke("get_state");
  if (initial && !get(app)) app.set(initial);
}

export function act(type, payload = {}) {
  const action = { type, ...payload };
  if (invoke) invoke("action", { action });
  else window.__vortoMock?.(action);
}

// A save carries the whole settings object. Edits from the last moments are carried along,
// because the snapshot may not show them yet and would undo them.
const recentEdits = new Map();
/** The settings including edits the snapshot may not show yet: build changes on these. */
export function currentSettings() {
  const now = Date.now();
  for (const [key, edit] of recentEdits) if (now - edit.at > 2000) recentEdits.delete(key);
  const settings = { ...get(app).settings };
  for (const [key, edit] of recentEdits) settings[key] = edit.value;
  return settings;
}
export function saveSettings(patch) {
  const now = Date.now();
  for (const [key, value] of Object.entries(patch)) recentEdits.set(key, { value, at: now });
  act("saveSettings", { settings: currentSettings() });
}

export async function startsHidden() {
  return invoke ? invoke("start_hidden") : false;
}

/** Whether the main window is on screen; always true in the browser mock. */
export async function windowVisible() {
  if (!isTauri) return true;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  return (await w.isVisible()) && !(await w.isMinimized());
}

export async function windowControl(kind) {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (kind === "minimize") w.minimize();
  if (kind === "maximize") w.toggleMaximize();
  if (kind === "close") w.close();
  // Through the app, which also wakes the page's rendering.
  if (kind === "show") await invoke("show_window");
}

/** Speech RMS to 0..1 on a decibel scale: -60 dB is silent, -12 dB is loud. */
export function normalizeLevel(rms) {
  const db = 20 * Math.log10(Math.max(rms, 1e-6));
  return Math.min(1, Math.max(0, (db + 60) / 48));
}

/** Recording clock: "m:ss", or "Ns left" in the last ten seconds before the limit. */
export function clock(elapsed, limit) {
  if (limit > 0 && limit - elapsed <= 10) return `${Math.max(0, limit - elapsed)}s left`;
  return `${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`;
}

/** Download progress without the model name, e.g. "141 of 670 MB". */
export function downloadText(s) {
  const detail = s?.detail ?? "";
  return detail.includes("·") ? detail.slice(detail.lastIndexOf("·") + 1).trim() : detail;
}
