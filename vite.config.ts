import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const chunkGroups = [
  { name: "ui", packages: ["naive-ui", "@vicons/material"] },
  { name: "tauri", packages: ["@tauri-apps/api", "@tauri-apps/plugin-store"] },
  { name: "openai", packages: ["openai"] },
];

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    strictPort: true,
    host: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: ["es2021", "chrome100", "safari15"],
    outDir: "dist",
    emptyOutDir: true,
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        advancedChunks: {
          groups: chunkGroups.map(({ name, packages }) => ({
            name,
            test: (id) =>
              packages.some((pkg) => id.includes(`/node_modules/${pkg}/`)),
          })),
        },
      },
    },
  },
});
