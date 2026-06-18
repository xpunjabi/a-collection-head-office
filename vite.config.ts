import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  // Tauri expects a fixed port, and it should fail if that port is already in use
  server: {
    port: 1420,
    strictPort: true,
    host: true,
  },
  // Prevent vite from obscuring rust errors
  clearScreen: false,
});
