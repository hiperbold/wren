import { Fragment, useState } from "react";
import { useTranslation } from "react-i18next";
import { Keyboard } from "lucide-react";
import { cn } from "@/lib/utils";
import { baseKeyFromCode, modifiersFrom, prettyKey } from "@/lib/format";

/**
 * Shortcut recorder: the WHOLE BOX is the click target (a <button>). Clicking
 * enters "recording" (animated ring + live prompt); the combination is
 * confirmed on the first NON-modifier key. Blur cancels and keeps the value.
 * Escape does NOT cancel (Escape alone is a valid value — the cancel default).
 * Final validation is the backend's (the plugin's parse runs on save).
 */
export function ShortcutRecorder({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation("common");
  const [recording, setRecording] = useState(false);
  const [heldMods, setHeldMods] = useState<string[]>([]);

  const onKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    const mods = modifiersFrom(e);
    const base = baseKeyFromCode(e.code);
    if (!base) {
      setHeldMods(mods);
      return;
    }
    onChange([...mods, base].join("+"));
    setRecording(false);
  };

  const onKeyUp = (e: React.KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    setHeldMods(modifiersFrom(e));
  };

  const parts = value.trim() ? value.split("+") : [];

  return (
    <button
      type="button"
      aria-label={
        recording ? t("recorder.recordingAria") : t("recorder.idleAria")
      }
      className={cn(
        "group flex min-h-10 w-full items-center gap-2.5 rounded-sm border bg-surface-2 px-3 py-2 text-left transition-colors cursor-pointer",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        recording
          ? "border-ring animate-pulse-ring"
          : "border-border hover:border-border-strong",
      )}
      onClick={() => {
        setHeldMods([]);
        setRecording(true);
      }}
      onKeyDown={onKeyDown}
      onKeyUp={onKeyUp}
      onBlur={() => setRecording(false)}
    >
      <Keyboard
        className={cn(
          "size-4 shrink-0",
          recording ? "text-primary" : "text-subtle-foreground",
        )}
      />
      {recording ? (
        <span className="text-sm text-muted-foreground">
          {heldMods.length > 0
            ? `${heldMods.map(prettyKey).join(" + ")} + …`
            : t("recorder.pressCombo")}
        </span>
      ) : parts.length > 0 ? (
        <span className="flex flex-1 items-center gap-1.5">
          {parts.map((part, i) => (
            <Fragment key={i}>
              {i > 0 && <span className="text-subtle-foreground">+</span>}
              <Keycap>{prettyKey(part)}</Keycap>
            </Fragment>
          ))}
        </span>
      ) : (
        <span className="text-sm text-muted-foreground">
          {t("recorder.none")}
        </span>
      )}
      {!recording && (
        <span className="ml-auto text-xs text-subtle-foreground opacity-0 transition-opacity group-hover:opacity-100">
          {t("recorder.hint")}
        </span>
      )}
    </button>
  );
}

/** Consistent keycap (reusable on any screen that shows keys). */
export function Keycap({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex min-w-6 items-center justify-center rounded-sm border border-border-strong border-b-2 bg-surface px-2 py-0.5 text-xs font-medium text-foreground">
      {children}
    </kbd>
  );
}
