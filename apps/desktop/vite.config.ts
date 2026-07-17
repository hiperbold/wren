import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Fixed port: tauri.conf.json points devUrl to 1420.
export default defineConfig(({ mode }) => {
  // `mock` mode (npm run dev:mock): runs the Settings window in a plain browser,
  // without a Tauri backend. We swap the `@tauri-apps/api` modules for local
  // mocks that return fake data. Does not affect the real dev/build.
  const isMock = mode === "mock";

  // Alias `@` → src (shadcn/UI contract). Vite's matcher requires a path
  // boundary ("@/…" or exact), so it does NOT collide with "@tauri-apps/…".
  const alias: Record<string, string> = {
    "@": fileURLToPath(new URL("./src", import.meta.url)),
  };
  if (isMock) {
    alias["@tauri-apps/api/core"] = fileURLToPath(
      new URL("./src/mock/core.ts", import.meta.url),
    );
    alias["@tauri-apps/api/event"] = fileURLToPath(
      new URL("./src/mock/event.ts", import.meta.url),
    );
  }

  return {
    plugins: [react(), tailwindcss()],
    clearScreen: false,
    resolve: { alias },
    server: {
      port: isMock ? 5173 : 1420,
      strictPort: false,
    },
    build: {
      outDir: "dist",
      target: "es2022",
    },
  };
});
