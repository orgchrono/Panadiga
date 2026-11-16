import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";
import tailwindcss from "tailwindcss";
import autoprefixer from "autoprefixer";

const tailwindConfig = {
  content: [
    resolve(import.meta.dirname, "./index.html"),
    resolve(import.meta.dirname, "./src/**/*.{ts,tsx}"),
  ],
  theme: {
    extend: {
      colors: {
        bench: {
          950: "#090d16",
          900: "#0f172a",
          850: "#131d33",
          800: "#1e293b",
        },
        diga: {
          dark: "#0b0f19",
          panel: "#111827",
          border: "#1f2937",
          accent: "#2563eb",
          accentHover: "#1d4ed8",
          emerald: "#10b981",
          amber: "#f59e0b",
          rose: "#ef4444",
          cyan: "#06b6d4",
        },
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "Courier New", "monospace"],
      },
    },
  },
  plugins: [],
};

const isAndroidApk =
  process.env.PANADIGA_TARGET === "android" ||
  process.argv.some(
    (arg, idx, arr) =>
      arg === "--mode=android" || (arg === "--mode" && arr[idx + 1] === "android")
  );

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  define: {
    __PANADIGA_ANDROID_APK__: JSON.stringify(isAndroidApk),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: {
      "@": resolve(import.meta.dirname, "./src"),
    },
  },
  optimizeDeps: {
    include: [
      "react",
      "react-dom",
      "react-i18next",
      "i18next",
      "@radix-ui/react-dialog",
      "@radix-ui/react-slot",
      "@radix-ui/react-tabs",
      "framer-motion",
      "lucide-react",
    ],
  },
  css: {
    postcss: {
      plugins: [tailwindcss(tailwindConfig), autoprefixer()],
    },
  },
  build: {
    outDir: isAndroidApk ? "dist-android" : "dist",
    emptyOutDir: true,
    target: "es2020",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      output: {
        manualChunks(id) {
          const norm = id.replace(/\\/g, "/");
          if (norm.includes("/src/locales/pt-BR.json")) {
            return "locales-pt-br";
          }
          if (norm.includes("/src/locales/en-US.json")) {
            return "locales-en-us";
          }
          if (norm.includes("node_modules")) {
            if (norm.includes("lucide-react")) {
              return "vendor-icons";
            }
            if (norm.includes("@tauri-apps")) {
              return "vendor-tauri";
            }
            if (
              norm.includes("framer-motion") ||
              norm.includes("motion-dom") ||
              norm.includes("motion-utils")
            ) {
              return "vendor-motion";
            }
            if (
              norm.includes("@reduxjs") ||
              norm.includes("react-redux") ||
              norm.includes("/redux/") ||
              norm.includes("/immer/") ||
              norm.includes("/reselect/") ||
              norm.includes("use-sync-external-store")
            ) {
              return "vendor-state";
            }
            if (
              norm.includes("@radix-ui") ||
              norm.includes("sonner") ||
              norm.includes("class-variance-authority") ||
              norm.includes("/clsx/") ||
              norm.includes("tailwind-merge")
            ) {
              return "vendor-ui";
            }
            if (norm.includes("i18next") || norm.includes("react-i18next")) {
              return "vendor-i18n";
            }
            if (
              norm.includes("/react/") ||
              norm.includes("/react-dom/") ||
              norm.includes("/scheduler/")
            ) {
              return "vendor-react";
            }
          }
        },
      },
    },
  },
});
