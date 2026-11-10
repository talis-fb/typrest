// Brand typefaces, self-hosted through Fontsource; latin subsets only to keep the embedded UI small.
import "@fontsource-variable/fraunces/opsz.css";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "./app.css";
import App from "./App.svelte";
import { mount } from "svelte";

mount(App, {
  target: document.getElementById("app")!,
});
