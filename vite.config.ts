import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

const vitePort = Number(process.env.DIMWEAVE_VITE_PORT) || 1420;

/** Globs ignored by the Vite dev-server file watcher. */
export const WATCH_IGNORED_GLOBS: string[] = [
  "**/.worktrees/**",
  "**/worktrees/**",
];

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  clearScreen: false,
  server: {
    port: vitePort,
    strictPort: true,
    hmr: true,
    watch: { ignored: WATCH_IGNORED_GLOBS },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "esnext",
    minify: false,
    sourcemap: true,
    rollupOptions: {
      output: {
        entryFileNames: "assets/[name].js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/[name][extname]",
        manualChunks: {
          markdown: ["react-markdown", "remark-gfm"],
        },
      },
    },
  },
});
