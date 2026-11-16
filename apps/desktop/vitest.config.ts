import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

export default defineConfig({
  plugins: [react()],
  define: {
    __PANADIGA_ANDROID_APK__: false,
  },
  test: {
    globals: true,
    pool: "threads",
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    exclude: ["e2e/**", "node_modules/**", "dist/**", "dist-android/**"],
    setupFiles: [resolve(import.meta.dirname, "./src/test/setup.ts")],
  },
  resolve: {
    alias: {
      "@": resolve(import.meta.dirname, "./src"),
    },
  },
});
