import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

const app = mount(App, { target: document.getElementById("app")! });

// Desktop app: no browser context menu
window.addEventListener("contextmenu", (e) => {
  if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
});

export default app;
