import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import { closkell } from "@closkell/vite-plugin";

export default defineConfig({
  plugins: [
    tailwindcss(),
    closkell({
      entry: "src/app.clsk",
      rootId: "root",
      css: "src/styles.css",
      vendorRuntime: true,
      sourceMap: true
    })
  ],
  server: {
    host: "127.0.0.1",
    port: 5174
  },
  preview: {
    host: "127.0.0.1",
    port: 4174
  },
  build: {
    sourcemap: true,
    target: "es2022"
  }
});
