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
      manifestPath: "../../Cargo.toml",
      vendorRuntime: true,
      sourceMap: false
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
    sourcemap: false,
    target: "es2022"
  }
});
