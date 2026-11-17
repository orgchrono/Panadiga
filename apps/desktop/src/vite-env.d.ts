/// <reference types="vite/client" />

// Seal ImportMetaEnv against fallback `[key: string]: any` indexing (`vite/types/importMeta.d.ts`).
interface ViteTypeOptions {
  strictImportMetaEnv: unknown;
}
