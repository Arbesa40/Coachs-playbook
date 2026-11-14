import "$shared/styles.css";
import "./app.css";
import { mount } from "svelte";
import { initTheme } from "$shared/theme.svelte.js";

initTheme();
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app") });
