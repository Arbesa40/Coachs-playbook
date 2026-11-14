import "$shared/styles.css";
import "./app.css";
import { mount } from "svelte";
import { initTheme } from "$shared/theme.svelte.js";
import App from "./App.svelte";
import VideoWindow from "./VideoWindow.svelte";
import BoardWindow from "./BoardWindow.svelte";
import CompareWindow from "./CompareWindow.svelte";

// One bundle serves the main window and the #video, #board and #compare second-screen windows.
initTheme();
const Root = { "#video": VideoWindow, "#board": BoardWindow, "#compare": CompareWindow }[location.hash] ?? App;
export default mount(Root, { target: document.getElementById("app") });
