import "@fontsource-variable/inter";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { applyTheme } from "./lib/api.js";
import Hud from "./Hud.svelte";
import Flash from "./Flash.svelte";
import LogoPreview from "./lib/LogoPreview.svelte";

const hud = location.hash === "#hud";
const flash = location.hash === "#flash";
const query = new URLSearchParams(location.search);
const logos = query.has("logos");
// Both overlays have a transparent page.
document.documentElement.classList.toggle("hud", hud || flash);
applyTheme();
mount(hud ? Hud : flash ? Flash : logos ? LogoPreview : App, { target: document.getElementById("app") });
