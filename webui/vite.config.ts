import path from "node:path";
import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

const webuiDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  base: "/_webui/",
  plugins: [tailwindcss(), svelte()],
  resolve: {
    // brand/daisyui/typrest-themes.css loads `daisyui/theme` from outside webui/, where Node's
    // lookup finds no node_modules; point it at this package's copy.
    alias: { daisyui: path.join(webuiDir, "node_modules/daisyui") },
  },
  server: {
    // The logos, favicon, and themes are read from the brand kit at the repository root.
    fs: { allow: [".", "../brand"] },
    proxy: {
      "/_api": "http://127.0.0.1:8080",
      "^/(?!_)": "http://127.0.0.1:8080",
    },
  },
});
