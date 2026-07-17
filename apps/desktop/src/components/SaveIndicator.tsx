import { useTranslation } from "react-i18next";
import { Check, Loader2, TriangleAlert } from "lucide-react";
import { useSettingsStore } from "@/lib/store";
import { cn } from "@/lib/utils";

/**
 * Subtle autosave feedback, anchored to the bottom-right corner. Shows on
 * "saving"/"saved"/"error" and disappears on its own when it returns to "idle"
 * (the store clears "saved" after ~1.6s). Reads state straight from the store —
 * takes no props.
 */
export function SaveIndicator() {
  const { t } = useTranslation("common");
  const saveState = useSettingsStore((s) => s.saveState);
  const saveError = useSettingsStore((s) => s.saveError);

  if (saveState === "idle") return null;

  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        "pointer-events-none fixed bottom-5 right-5 z-50 flex items-center gap-2 rounded-md border px-3 py-2 text-sm shadow-lg animate-fade-in",
        saveState === "error"
          ? "border-danger/40 bg-danger-bg text-danger"
          : "border-border bg-popover text-foreground",
      )}
    >
      {saveState === "saving" && (
        <>
          <Loader2 className="size-4 animate-spin text-muted-foreground" />
          <span className="text-muted-foreground">{t("save.saving")}</span>
        </>
      )}
      {saveState === "saved" && (
        <>
          <Check className="size-4 text-success" />
          <span>{t("save.saved")}</span>
        </>
      )}
      {saveState === "error" && (
        <>
          <TriangleAlert className="size-4" />
          <span>
            {saveError
              ? t("save.errorDetail", { error: saveError })
              : t("save.error")}
          </span>
        </>
      )}
    </div>
  );
}
