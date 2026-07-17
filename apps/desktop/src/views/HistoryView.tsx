import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { Check, Copy, Inbox, RotateCw, Search } from "lucide-react";
import { SectionHeader } from "@/components/SectionHeader";
import { HistoryStatusBadge } from "@/components/StatusBadge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useHistory, useRetryTranscription } from "@/lib/queries";
import { formatSecs, formatWhen } from "@/lib/format";
import i18n from "@/i18n";
import type { HistoryEntry } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** Midnight (local) of the timestamp — grouping key by day. */
function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** Day header label: "Today"/"Yesterday" or the full date. */
function dayLabel(ms: number, t: TFunction): string {
  const diff = Math.round((startOfDay(Date.now()) - startOfDay(ms)) / 86_400_000);
  if (diff === 0) return t("day.today");
  if (diff === 1) return t("day.yesterday");
  return new Date(ms).toLocaleDateString(i18n.language || "en", {
    weekday: "long",
    day: "2-digit",
    month: "long",
  });
}

/** Just the time (the day is already in the sticky header). */
function timeLabel(ms: number): string {
  return new Date(ms).toLocaleTimeString(i18n.language || "en", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Separador "·" das metadados. */
function Dot() {
  return <span className="text-border-strong">·</span>;
}

/** Metadata row for an entry (time · provider · duration · response), in
 * `tabular-nums` so the numbers line up. */
function EntryMeta({ t }: { t: HistoryEntry }) {
  const { t: tr } = useTranslation("history");
  const trimmed =
    t.recorded_duration_ms != null &&
    t.recorded_duration_ms !== t.audio_duration_ms;
  return (
    <div className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-xs text-muted-foreground tabular-nums">
      <span title={formatWhen(t.created_at_ms)}>{timeLabel(t.created_at_ms)}</span>
      {t.status === "done" && (
        <>
          <Dot />
          <span>{t.provider_id}</span>
        </>
      )}
      <Dot />
      {trimmed ? (
        <span>
          {tr("meta.trimmed", {
            recorded: formatSecs(t.recorded_duration_ms!),
            sent: formatSecs(t.audio_duration_ms),
          })}
        </span>
      ) : (
        <span>{tr("meta.audio", { secs: formatSecs(t.audio_duration_ms) })}</span>
      )}
      {t.status === "done" && (
        <>
          <Dot />
          <span>{tr("meta.latency", { ms: t.latency_ms })}</span>
        </>
      )}
    </div>
  );
}

/** A history card. Failures stand out at a glance thanks to the `danger` color
 * on the card ITSELF; the Copy/Resend actions only appear on hover/focus (less
 * constant visual weight), but stay reachable by keyboard. */
function HistoryCard({
  t,
  copied,
  retrying,
  retryDisabled,
  onCopy,
  onRetry,
}: {
  t: HistoryEntry;
  copied: boolean;
  retrying: boolean;
  retryDisabled: boolean;
  onCopy: (t: HistoryEntry) => void;
  onRetry: (t: HistoryEntry) => void;
}) {
  const { t: tr } = useTranslation("history");
  const failed = t.status === "failed";

  return (
    <div
      className={cn(
        "group relative rounded-lg border p-4 transition-colors",
        failed
          ? "border-danger/40 bg-danger-bg"
          : "border-border bg-card hover:border-border-strong",
      )}
    >
      <div className="mb-2 flex items-center justify-between gap-3">
        <HistoryStatusBadge status={t.status} />
        {/* Actions revealed on hover/focus. `group-focus-within` keeps them
            visible when a button receives keyboard focus. */}
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
          {failed ? (
            // The card already carries the `danger`; the button is the neutral
            // recovery CTA (avoids "red on red").
            t.audio_path && (
              <Button
                variant="secondary"
                size="sm"
                disabled={retryDisabled}
                onClick={() => onRetry(t)}
                title={tr("card.retryTitle")}
              >
                <RotateCw className={retrying ? "animate-spin" : undefined} />
                {retrying ? tr("card.retrying") : tr("card.retry")}
              </Button>
            )
          ) : (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onCopy(t)}
              title={tr("card.copyTitle")}
            >
              {copied ? <Check className="text-success" /> : <Copy />}
              {copied ? tr("card.copied") : tr("card.copy")}
            </Button>
          )}
        </div>
      </div>

      {failed ? (
        <p className="text-sm font-medium text-danger">
          {tr("card.failed")}
          {t.error && (
            <span className="mt-0.5 block text-xs font-normal text-muted-foreground">
              {t.error}
            </span>
          )}
        </p>
      ) : (
        <p className="text-sm leading-relaxed text-foreground">{t.text}</p>
      )}

      <div className="mt-2">
        <EntryMeta t={t} />
      </div>
    </div>
  );
}

// Dictation history: list read via useHistory, grouped by day (sticky header),
// with failures in a `danger` card and Copy/Resend actions on hover.
export default function HistoryView() {
  const { t } = useTranslation("history");
  const { data, isLoading } = useHistory();
  const retry = useRetryTranscription();
  const [query, setQuery] = useState("");
  const [copiedId, setCopiedId] = useState<number | null>(null);
  const copyTimer = useRef<number | undefined>(undefined);

  const entries = data ?? [];

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter(
      (e) =>
        e.text.toLowerCase().includes(q) ||
        e.provider_id.toLowerCase().includes(q) ||
        (e.error ?? "").toLowerCase().includes(q),
    );
  }, [entries, query]);

  // Entries already arrive sorted (most recent first); group them in sequence.
  const groups = useMemo(() => {
    const out: { key: number; label: string; items: HistoryEntry[] }[] = [];
    for (const e of filtered) {
      const key = startOfDay(e.created_at_ms);
      const last = out[out.length - 1];
      if (last && last.key === key) last.items.push(e);
      else out.push({ key, label: dayLabel(e.created_at_ms, t), items: [e] });
    }
    return out;
  }, [filtered, t]);

  const flashCopied = (id: number) => {
    setCopiedId(id);
    window.clearTimeout(copyTimer.current);
    copyTimer.current = window.setTimeout(() => setCopiedId(null), 1500);
  };

  const copyEntry = (t: HistoryEntry) => {
    navigator.clipboard?.writeText(t.text).catch(() => {});
    flashCopied(t.created_at_ms);
  };

  // Resend: transcribe again and copy the text (focus is here, not on the
  // target app, so it doesn't paste on its own) — preserves the old behavior.
  const retryEntry = (t: HistoryEntry) => {
    retry.mutate(t.created_at_ms, {
      onSuccess: (text) => {
        navigator.clipboard?.writeText(text).catch(() => {});
        flashCopied(t.created_at_ms);
      },
    });
  };

  return (
    <div className="max-w-[760px]">
      <SectionHeader
        title={t("header.title")}
        description={t("header.description")}
      />

      {entries.length > 0 && (
        <div className="relative mb-5">
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-subtle-foreground" />
          <Input
            type="search"
            placeholder={t("search.placeholder")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="pl-9"
          />
        </div>
      )}

      {isLoading && entries.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("loading")}</p>
      ) : entries.length === 0 ? (
        <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-border bg-surface/40 px-6 py-14 text-center">
          <Inbox className="size-8 text-subtle-foreground" />
          <div className="space-y-1">
            <p className="text-sm font-medium text-foreground">
              {t("empty.title")}
            </p>
            <p className="text-xs text-muted-foreground">
              {t("empty.description")}
            </p>
          </div>
        </div>
      ) : filtered.length === 0 ? (
        <p className="py-8 text-center text-sm text-muted-foreground">
          {t("search.noResults", { query })}
        </p>
      ) : (
        <div className="space-y-6">
          {groups.map((g) => (
            <section key={g.key}>
              <h2 className="sticky top-0 z-10 mb-2 -mx-1 bg-background/90 px-1 py-1 text-xs font-semibold capitalize tracking-wide text-subtle-foreground backdrop-blur">
                {g.label}
              </h2>
              <div className="space-y-3">
                {g.items.map((t) => (
                  <HistoryCard
                    key={t.created_at_ms}
                    t={t}
                    copied={copiedId === t.created_at_ms}
                    retrying={retry.isPending && retry.variables === t.created_at_ms}
                    retryDisabled={retry.isPending}
                    onCopy={copyEntry}
                    onRetry={retryEntry}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
