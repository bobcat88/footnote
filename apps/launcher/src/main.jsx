import "@fontsource/plus-jakarta-sans/600.css";
import "@fontsource/plus-jakarta-sans/700.css";
import "@fontsource/hanken-grotesk/400.css";
import "@fontsource/hanken-grotesk/500.css";
import "@fontsource/hanken-grotesk/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "./styles.css";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createRoot } from "react-dom/client";
import App from "./App.jsx";
import { inApp } from "./data.js";
import Mini from "./Mini.jsx";

// One page, two windows: the `mini` window (tauri.conf.json) shows mini mode; in a browser, the `#/mini` route does.
const mini = inApp ? getCurrentWindow().label === "mini" : location.hash.startsWith("#/mini");
createRoot(document.getElementById("root")).render(mini ? <Mini /> : <App />);
