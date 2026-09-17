import "@fontsource-variable/inter";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import Hud from "./Hud.svelte";
import LogoPreview from "./lib/LogoPreview.svelte";

const hud = location.hash === "#hud";
const query = new URLSearchParams(location.search);
const logos = query.has("logos");
document.documentElement.classList.toggle("hud", hud);
mount(hud ? Hud : logos ? LogoPreview : App, { target: document.getElementById("app") });
