import { defineConfig, type Plugin } from "vite";
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

const FORBIDDEN_DESKTOP_TOKENS = [
  "ircommander",
  "panasonic_ir_catalog",
  "__PANADIGA_IR_BRIDGE__",
] as const;

function stripDesktopIrLocalesPlugin(strip: boolean): Plugin {
  return {
    name: "panadiga-strip-desktop-ir-locales",
    load(id: string) {
      if (!strip) {
        return null;
      }
      const norm = id.replace(/\\/g, "/");
      if (/\/src\/locales\/ir\/(en-US|pt-BR)\.json$/.test(norm)) {
        return "{}";
      }
      return null;
    },
    generateBundle(_options, bundle) {
      if (!strip) {
        return;
      }
      for (const [fileName, output] of Object.entries(bundle)) {
        const content =
          output.type === "chunk"
            ? output.code
            : typeof output.source === "string"
              ? output.source
              : "";
        for (const token of FORBIDDEN_DESKTOP_TOKENS) {
          if (content.includes(token)) {
            this.error(
              `LEAK: Desktop bundle contains Android-only token ${token} in ${fileName}`
            );
          }
        }
      }
    },
  };
}

// https://vitejs.dev/config/
export default defineConfig(({ command, mode }) => ({
  plugins: [react(), stripDesktopIrLocalesPlugin(command === "build" && !isAndroidApk)],
  define:
    isAndroidApk && mode !== "android"
      ? { "import.meta.env.MODE": JSON.stringify("android") }
      : undefined,
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    warmup: {
      clientFiles: [
        "./src/main.tsx",
        "./src/App.tsx",
        "./src/views/ExpressView.tsx",
        "./src/views/UnbrickView.tsx",
      ],
    },
  },
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: {
      "@": resolve(import.meta.dirname, "./src"),
    },
  },
  optimizeDeps: {
    include: [
      "@opentelemetry/api",
      "@radix-ui/react-dialog",
      "@radix-ui/react-slot",
      "@radix-ui/react-tabs",
      "@reduxjs/toolkit",
      "@tauri-apps/api/core",
      "@tauri-apps/api/event",
      "class-variance-authority",
      "clsx",
      "framer-motion",
      "i18next",
      "lucide-react",
      "pino",
      "react",
      "react-dom",
      "react-dom/client",
      "react-i18next",
      "react-redux",
      "sonner",
      "tailwind-merge",
      "zod",
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
          if (
            norm.includes("/src/locales/pt-BR/") ||
            norm.includes("/src/locales/ir/pt-BR.json")
          ) {
            return "locales-pt-br";
          }
          if (
            norm.includes("/src/locales/en-US/") ||
            norm.includes("/src/locales/ir/en-US.json")
          ) {
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
            if (norm.includes("/zod/")) {
              return "vendor-validation";
            }
            if (
              norm.includes("/pino/") ||
              norm.includes("@pinojs/") ||
              norm.includes("quick-format") ||
              norm.includes("@opentelemetry/")
            ) {
              return "vendor-telemetry";
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
}));
