import { createApp } from "vue";
import App from "./App.vue";
import "./code/syntaxHighlight.css";

// Suppress the webview menu before editor or other component handlers run.
window.addEventListener("contextmenu", (event) => event.preventDefault(), {
  capture: true,
});

createApp(App).mount("#app");
