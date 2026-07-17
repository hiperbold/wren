import { Suspense, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { TooltipProvider } from "@/components/ui/tooltip";
import { SaveIndicator } from "@/components/SaveIndicator";
import { useSettingsQuery } from "@/lib/queries";
import { useSettingsStore } from "@/lib/store";
import { NAV } from "./nav";
import { Sidebar } from "./Sidebar";

/** App shell: sidebar + content area (with per-view Suspense) + the "Saved"
 * toaster. Hydrates the settings store from the initial `get_settings`. */
export function App() {
  const { t } = useTranslation("common");
  const [active, setActive] = useState(NAV[0].id);
  const settingsQuery = useSettingsQuery();
  const hydrate = useSettingsStore((s) => s.hydrate);
  const hydrated = useSettingsStore((s) => s.settings !== null);

  // Seed the store once when the initial value arrives (never overwrite edits).
  useEffect(() => {
    if (settingsQuery.data) hydrate(settingsQuery.data);
  }, [settingsQuery.data, hydrate]);

  const Active = NAV.find((n) => n.id === active)?.Component ?? NAV[0].Component;

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
        <Sidebar active={active} onSelect={setActive} />
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[840px] px-8 py-8">
            {settingsQuery.isError ? (
              <p className="text-sm text-danger">
                {t("app.loadSettingsError", {
                  error: String(settingsQuery.error),
                })}
              </p>
            ) : !hydrated ? (
              <Loading />
            ) : (
              <Suspense fallback={<Loading />}>
                {/* Short fade (~180ms) on every section change: key = section
                    remounts the wrapper → the CSS animation plays on mount.
                    Zero-dep (`animate-fade-in` in index.css), respects
                    prefers-reduced-motion globally. */}
                <div key={active} className="animate-fade-in">
                  <Active />
                </div>
              </Suspense>
            )}
          </div>
        </main>
        <SaveIndicator />
      </div>
    </TooltipProvider>
  );
}

function Loading() {
  const { t } = useTranslation("common");
  return (
    <div className="flex items-center gap-2 py-16 text-sm text-muted-foreground">
      <Loader2 className="size-4 animate-spin" />
      {t("app.loading")}
    </div>
  );
}
