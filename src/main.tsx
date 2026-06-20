import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { HoverPopover } from "./components/HoverPopover";
import { isTauriReady } from "./tauriReady";
import "./styles/theme.css";
import "./styles/panel.css";

// The hover popover loads this same bundle in a separate window; render the
// compact glance there instead of the full app.
const isHover = isTauriReady() && getCurrentWindow().label === "hover";

const root = document.getElementById("app");
if (!root) throw new Error("Stepwise: #app root not found");

createRoot(root).render(
  <StrictMode>{isHover ? <HoverPopover /> : <App />}</StrictMode>,
);
