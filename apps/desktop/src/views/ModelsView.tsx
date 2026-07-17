import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Download, HardDriveDownload, Star, Trash2 } from "lucide-react";
import { SectionHeader } from "@/components/SectionHeader";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useSettingsStore } from "@/lib/store";
import {
  useDeleteModel,
  useDownloadModel,
  useEmbeddedCatalog,
  useLocalModels,
} from "@/lib/queries";
import { onDownloadProgress } from "@/lib/tauri";
import type { EmbeddedModel } from "@/lib/tauri";
import { formatBytes, prettyLang } from "@/lib/format";
import StarBorder from "@/components/reactbits/StarBorder";
import { cn } from "@/lib/utils";

/** Recommended model — gets a subtle "Recommended" highlight. */
const RECOMMENDED_ID = "parakeet-v3";

/** UI state of an in-progress (or failed) download, per model. */
interface DownloadState {
  downloaded: number;
  total: number;
  error?: string;
}

/**
 * VIEW: Models (offline). Embedded-engine catalog crossed with the already
 * downloaded ones; each model in a Card with download (real per-event
 * progress), activate (builds a kind:"embedded" provider + saveNow), and
 * remove (confirmation Dialog — multi-GB files). The audio never leaves the
 * machine.
 */
export default function ModelsView() {
  const { t } = useTranslation("models");
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const saveNow = useSettingsStore((s) => s.saveNow);

  const catalogQuery = useEmbeddedCatalog();
  const localQuery = useLocalModels();
  const downloadModel = useDownloadModel();
  const deleteModel = useDeleteModel();

  // Progresso de todos os downloads, por id do modelo. Semeado em 0% ao iniciar.
  const [downloads, setDownloads] = useState<Record<string, DownloadState>>({});
  const [pendingDelete, setPendingDelete] = useState<EmbeddedModel | null>(null);
  const [activating, setActivating] = useState<string | null>(null);
  // Just-activated model — gets the StarBorder highlight for ~1.5s (momentary).
  const [glowId, setGlowId] = useState<string | null>(null);

  // A single listener for the progress of ALL downloads; filters by id.
  useEffect(() => {
    let un: undefined | (() => void);
    let disposed = false;
    onDownloadProgress((p) => {
      if (p.error) {
        setDownloads((d) => ({
          ...d,
          [p.id]: { downloaded: p.downloaded, total: p.total, error: p.error },
        }));
        return;
      }
      if (p.done) {
        setDownloads((d) => without(d, p.id));
        void localQuery.refetch();
        return;
      }
      setDownloads((d) => ({
        ...d,
        [p.id]: { downloaded: p.downloaded, total: p.total },
      }));
    }).then((fn) => {
      if (disposed) fn();
      else un = fn;
    });
    return () => {
      disposed = true;
      un?.();
    };
    // localQuery.refetch is stable; register a single time on mount.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!settings) return null;

  const catalog = catalogQuery.data;
  const local = localQuery.data ?? [];
  const activeProvider = settings.providers.find(
    (p) => p.id === settings.active_provider_id,
  );

  const startDownload = (m: EmbeddedModel) => {
    // Seed the bar at 0% right away — the UI reacts before the first event.
    setDownloads((d) => ({
      ...d,
      [m.id]: { downloaded: 0, total: m.sizeBytes },
    }));
    downloadModel.mutate(m.id, {
      // done/error usually already arrive via event; the mutation is the safety
      // net (clears the bar and invalidates the downloaded list on resolve).
      onSuccess: () => setDownloads((d) => without(d, m.id)),
      onError: (e) =>
        setDownloads((d) => ({
          ...d,
          [m.id]: {
            downloaded: d[m.id]?.downloaded ?? 0,
            total: d[m.id]?.total ?? m.sizeBytes,
            error: String(e),
          },
        })),
    });
  };

  // Activate: builds (or reuses) a kind:"embedded" provider and makes it
  // active. Instant action → update + saveNow (doesn't wait for the debounce).
  // Mirrors activateEmbedded from SettingsPage.
  const activate = (m: EmbeddedModel) => {
    setActivating(m.id);
    const providerId = `embedded-${m.id}`;
    update((s) => {
      const providers = s.providers.some((p) => p.id === providerId)
        ? s.providers
        : [
            ...s.providers,
            {
              id: providerId,
              label: m.label,
              kind: "embedded",
              base_url: "",
              api_key: null,
              model: m.id,
              sends_audio_externally: false,
            },
          ];
      return { ...s, providers, active_provider_id: providerId };
    });
    saveNow();
    // saveNow is synchronous on trigger; releasing the button on the next tick
    // is enough.
    window.setTimeout(() => setActivating(null), 300);
    // Momentary highlight (React Bits StarBorder) on the just-activated card.
    setGlowId(m.id);
    window.setTimeout(() => setGlowId((cur) => (cur === m.id ? null : cur)), 1500);
  };

  const confirmDelete = () => {
    const m = pendingDelete;
    if (!m) return;
    deleteModel.mutate(m.id, {
      onSettled: () => setPendingDelete(null),
    });
  };

  return (
    <div className="max-w-[560px]">
      <SectionHeader title={t("title")} description={t("description")} />

      <p className="mb-4 text-sm text-muted-foreground leading-relaxed">
        {t("intro.before")}{" "}
        <span className="font-medium text-success">{t("intro.offline")}</span>
        {t("intro.after")}
      </p>

      {catalogQuery.isLoading && (
        <p className="text-sm text-muted-foreground">{t("catalog.loading")}</p>
      )}

      {!catalogQuery.isLoading && (!catalog || catalog.length === 0) && (
        <Card>
          <CardContent>
            <p className="text-sm text-muted-foreground">
              {catalogQuery.isError
                ? t("catalog.unavailable")
                : t("catalog.empty")}
            </p>
          </CardContent>
        </Card>
      )}

      <div className="space-y-3">
        {catalog?.map((m) => {
          const isLocal = local.includes(m.id);
          const isActive =
            activeProvider?.kind === "embedded" &&
            activeProvider.model === m.id;
          return (
            <ModelCard
              key={m.id}
              model={m}
              isLocal={isLocal}
              isActive={isActive}
              recommended={m.id === RECOMMENDED_ID}
              download={downloads[m.id]}
              activating={activating === m.id}
              glow={glowId === m.id}
              onDownload={() => startDownload(m)}
              onActivate={() => activate(m)}
              onRemove={() => setPendingDelete(m)}
            />
          );
        })}
      </div>

      <Dialog
        open={!!pendingDelete}
        onOpenChange={(o) => !o && setPendingDelete(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("delete.title", { label: pendingDelete?.label })}
            </DialogTitle>
            <DialogDescription>
              {t("delete.body", {
                size: pendingDelete ? formatBytes(pendingDelete.sizeBytes) : "",
              })}
              {pendingDelete &&
              activeProvider?.kind === "embedded" &&
              activeProvider.model === pendingDelete.id
                ? t("delete.active")
                : t("delete.reDownload")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPendingDelete(null)}>
              {t("delete.cancel")}
            </Button>
            <Button
              variant="destructive"
              disabled={deleteModel.isPending}
              onClick={confirmDelete}
            >
              <Trash2 className="size-4" />
              {deleteModel.isPending ? t("delete.removing") : t("delete.remove")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

/* -------------------------------- card ------------------------------------- */

function ModelCard({
  model,
  isLocal,
  isActive,
  recommended,
  download,
  activating,
  glow,
  onDownload,
  onActivate,
  onRemove,
}: {
  model: EmbeddedModel;
  isLocal: boolean;
  isActive: boolean;
  recommended: boolean;
  download?: DownloadState;
  activating: boolean;
  glow: boolean;
  onDownload: () => void;
  onActivate: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation("models");
  const downloading = !!download && !download.error;
  const failed = !!download?.error;
  const pct =
    download && download.total > 0
      ? Math.min(100, Math.round((download.downloaded / download.total) * 100))
      : 0;

  return (
    <Card
      className={cn(
        "relative overflow-hidden",
        isActive && "border-success/50",
      )}
    >
      {glow && (
        <StarBorder overlay color="rgba(255, 178, 102, 0.9)" speed="1.4s" />
      )}
      <CardContent className="relative z-[1] space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-semibold">{model.label}</span>
              {recommended && (
                <Badge variant="accent">
                  <Star className="size-3" />
                  {t("badge.recommended")}
                </Badge>
              )}
              {isActive && (
                <Badge variant="success" className="animate-pop">
                  <Check className="size-3" />
                  {t("badge.active")}
                </Badge>
              )}
              {isLocal && !isActive && (
                <Badge variant="success">
                  <HardDriveDownload className="size-3" />
                  {t("badge.offline")}
                </Badge>
              )}
            </div>
            <p className="text-xs text-muted-foreground tabular-nums">
              {prettyLang(model.language)} ·{" "}
              <span className="font-medium text-foreground">
                {formatBytes(model.sizeBytes)}
              </span>
            </p>
          </div>

          {/* Card's main action (varies by state). */}
          <div className="shrink-0">
            {!isLocal && !downloading && (
              <Button size="sm" variant="primary" onClick={onDownload}>
                <Download className="size-4" />
                {failed ? t("card.retry") : t("card.download")}
              </Button>
            )}
            {isLocal && isActive && (
              <Button size="sm" variant="secondary" disabled>
                <Check className="size-4" />
                {t("badge.active")}
              </Button>
            )}
            {isLocal && !isActive && (
              <Button
                size="sm"
                variant="primary"
                disabled={activating}
                onClick={onActivate}
              >
                {activating ? t("card.activating") : t("card.activate")}
              </Button>
            )}
          </div>
        </div>

        {downloading && (
          <div className="space-y-1.5">
            <Progress value={pct} />
            <p className="text-xs text-muted-foreground tabular-nums">
              {t("progress.downloading", {
                pct,
                downloaded: formatBytes(download!.downloaded),
                total: formatBytes(download!.total),
              })}
            </p>
          </div>
        )}

        {failed && (
          <p className="text-sm text-danger" title={download!.error}>
            {t("progress.failed")}
          </p>
        )}

        {isLocal && (
          <div className="flex justify-end">
            <Button size="sm" variant="destructive" onClick={onRemove}>
              <Trash2 className="size-4" />
              {t("card.remove")}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/** Removes a key from a map without mutating it. */
function without<T>(map: Record<string, T>, key: string): Record<string, T> {
  if (!(key in map)) return map;
  const next = { ...map };
  delete next[key];
  return next;
}
