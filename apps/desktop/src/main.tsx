import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { queryClient } from "@/lib/query-client";
import { App } from "@/app/App";
import "@/i18n";
import "@/index.css";

// The web UI is only the settings window — the recording bubble is native
// (wgpu), no webview. In `--mode mock` Vite swaps @tauri-apps/api for the mocks;
// nothing here changes. The QueryClient is the instance shared with the store.
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
