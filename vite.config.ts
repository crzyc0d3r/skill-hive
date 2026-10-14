import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";

export const serverWatchIgnored = [
  "**/target/**",
  "**/src-tauri/target/**",
  "**/src-tauri/**",
  "crates/**",
  "**/crates/**",
  "graphify-out/**",
  "**/graphify-out/**",
  "automations/**",
  "**/automations/**",
  "**/docs/**",
  "**/packaging/**",
  "**/e2e/**",
  "**/scripts/**",
  "**/fixtures/**",
  "**/.github/**",
  "**/.dev-base/**",
  "**/dist/**",
];

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: serverWatchIgnored,
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
});
