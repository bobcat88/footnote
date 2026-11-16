import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri loads the dev server on a fixed port and keeps its own console output visible.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  // Every asset as a file: the window's policy (tauri.conf.json) allows fonts from the app itself, not data: URLs.
  build: { target: "es2022", assetsInlineLimit: 0 },
});
