import "./app.css";
import { mount } from "svelte";
import { initTheme } from "./lib/stores/theme.svelte";
import App from "./App.svelte";

initTheme();

const app = mount(App, {
  target: document.getElementById("app")!
});

export default app;