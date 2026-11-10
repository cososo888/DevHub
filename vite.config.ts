import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// @ts-expect-error node types are intentionally not a dependency of this prototype
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;
export default defineConfig(() => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1422 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] }
  }
}));
