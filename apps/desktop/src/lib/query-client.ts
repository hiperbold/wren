import { QueryClient } from "@tanstack/react-query";

/** Single QueryClient instance. Shared between the provider (main.tsx) and the
 * settings store (store.ts), so the autosave can sync the `get_settings` cache
 * after persisting without relying on the React context. */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5_000,
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});
