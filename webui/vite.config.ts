import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  base: "/webui/",
  plugins: [tailwindcss(), svelte()],
  server: {
    proxy: {
      "/_api": "http://127.0.0.1:8080",
      "/render": "http://127.0.0.1:8080",
    },
  },
});
