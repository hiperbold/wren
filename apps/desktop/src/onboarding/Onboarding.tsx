import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Download, Eye, EyeOff, Star } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Field } from "@/components/Field";
import { ShortcutRecorder } from "@/components/ShortcutRecorder";
import { useSettingsStore } from "@/lib/store";
import {
  useDownloadModel,
  useEmbeddedCatalog,
  useHardwareInfo,
  useLocalModels,
  useProviderPresets,
} from "@/lib/queries";
import { onDownloadProgress } from "@/lib/tauri";
import type { EmbeddedModel } from "@/lib/tauri";
import { formatBytes, prettyLang } from "@/lib/format";
import { cn } from "@/lib/utils";

/** Recommended model — mirrors ModelsView's RECOMMENDED_ID (not exported there). */
const RECOMMENDED_ID = "parakeet-v3";

type Engine = "local" | "remote";
type Step = "welcome" | "engine" | "local" | "remote" | "shortcut" | "test" | "done";

/**
 * First-run onboarding wizard. Full-screen — replaces the sidebar+content
 * shell entirely while `settings.onboarding_completed` is false (see App.tsx's
 * gate). Steps: welcome → engine choice → local-or-remote setup → shortcut →
 * test → done. Finishing OR skipping both flip the flag and saveNow (an
 * "instant action", same contract as ModelsView's activate()).
 */
export default function Onboarding() {
  const { t } = useTranslation("onboarding");
  const settings = useSettingsStore((s) => s.settings);
  const setField = useSettingsStore((s) => s.setField);
  const saveNow = useSettingsStore((s) => s.saveNow);

  const [step, setStep] = useState<Step>("welcome");
  const [engine, setEngine] = useState<Engine>("local");

  if (!settings) return null;

  // The branch step is "local" or "remote" depending on the engine choice —
  // always 6 steps total either way.
  const steps: Step[] = ["welcome", "engine", engine, "shortcut", "test", "done"];
  const currentIndex = steps.indexOf(step);
  const isDone = step === "done";

  const finish = () => {
    setField("onboarding_completed", true);
    saveNow();
  };

  const goNext = () => {
    const idx = steps.indexOf(step);
    if (idx < 0 || idx >= steps.length - 1) return;
    setStep(steps[idx + 1]);
  };

  const goBack = () => {
    const idx = steps.indexOf(step);
    if (idx <= 0) return;
    setStep(steps[idx - 1]);
  };

  const primaryLabel = step === "welcome" ? t("welcome.cta") : t("next");

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto flex min-h-full w-full max-w-[560px] flex-col justify-center px-8 py-12">
          <div key={step} className="animate-fade-in">
            {step === "welcome" && <WelcomeStep />}
            {step === "engine" && <EngineStep engine={engine} onSelect={setEngine} />}
            {step === "local" && <LocalStep />}
            {step === "remote" && <RemoteStep />}
            {step === "shortcut" && <ShortcutStep />}
            {step === "test" && <TestStep />}
            {step === "done" && <DoneStep onFinish={finish} />}
          </div>
        </div>
      </div>

      {!isDone && (
        <div className="flex items-center justify-between border-t border-border px-8 py-4">
          <button
            type="button"
            className="text-sm text-muted-foreground underline-offset-4 hover:underline"
            onClick={finish}
          >
            {t("skip")}
          </button>
          <span className="text-xs text-subtle-foreground">
            {t("stepOf", { current: currentIndex + 1, total: steps.length })}
          </span>
          <div className="flex gap-2">
            {currentIndex > 0 && (
              <Button variant="outline" onClick={goBack}>
                {t("back")}
              </Button>
            )}
            <Button variant="primary" onClick={goNext}>
              {primaryLabel}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

/* --------------------------------- steps --------------------------------- */

function StepHeading({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  return (
    <div className="mb-4 space-y-1.5">
      <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
      {description && (
        <p className="text-sm text-muted-foreground leading-relaxed">
          {description}
        </p>
      )}
    </div>
  );
}

function WelcomeStep() {
  const { t } = useTranslation("onboarding");
  return (
    <Card>
      <CardContent className="space-y-6 py-10 text-center">
        <h1 className="text-2xl font-semibold tracking-tight">
          {t("welcome.title")}
        </h1>
        <p className="text-sm text-muted-foreground leading-relaxed">
          {t("welcome.description")}
        </p>
      </CardContent>
    </Card>
  );
}

function EngineStep({
  engine,
  onSelect,
}: {
  engine: Engine;
  onSelect: (e: Engine) => void;
}) {
  const { t } = useTranslation("onboarding");
  return (
    <div>
      <StepHeading title={t("engine.title")} description={t("engine.description")} />
      <div className="space-y-3">
        <EngineCard
          selected={engine === "local"}
          title={t("engine.local.title")}
          description={t("engine.local.description")}
          onClick={() => onSelect("local")}
        />
        <EngineCard
          selected={engine === "remote"}
          title={t("engine.remote.title")}
          description={t("engine.remote.description")}
          onClick={() => onSelect("remote")}
        />
      </div>
      <details className="mt-4 rounded-sm border border-border bg-surface-2 px-3 py-2">
        <summary className="cursor-pointer text-sm font-medium text-foreground">
          {t("engine.technical")}
        </summary>
        <p className="mt-2 text-sm text-muted-foreground leading-relaxed">
          {t("engine.technicalNote")}
        </p>
      </details>
    </div>
  );
}

function EngineCard({
  selected,
  title,
  description,
  onClick,
}: {
  selected: boolean;
  title: string;
  description: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      onClick={onClick}
      className={cn(
        "w-full cursor-pointer rounded-lg border p-4 text-left transition-colors",
        selected
          ? "border-primary bg-primary/5"
          : "border-border bg-card hover:border-border-strong",
      )}
    >
      <div className="flex items-start gap-3">
        <span
          className={cn(
            "mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full border",
            selected ? "border-primary" : "border-border-strong",
          )}
        >
          {selected && <span className="size-2 rounded-full bg-primary" />}
        </span>
        <div className="space-y-1">
          <p className="font-semibold">{title}</p>
          <p className="text-sm text-muted-foreground leading-relaxed">
            {description}
          </p>
        </div>
      </div>
    </button>
  );
}

/** UI state of an in-progress (or failed) download, per model. Mirrors
 * ModelsView's DownloadState (not exported there). */
interface DownloadState {
  downloaded: number;
  total: number;
  error?: string;
}

function LocalStep() {
  const { t } = useTranslation("onboarding");
  const settings = useSettingsStore((s) => s.settings)!;
  const update = useSettingsStore((s) => s.update);
  const saveNow = useSettingsStore((s) => s.saveNow);

  const catalogQuery = useEmbeddedCatalog();
  const localQuery = useLocalModels();
  const downloadModel = useDownloadModel();
  const hardware = useHardwareInfo();

  const [downloads, setDownloads] = useState<Record<string, DownloadState>>({});
  const [activating, setActivating] = useState<string | null>(null);

  // Same single-listener pattern as ModelsView: register once on mount.
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const catalog = catalogQuery.data;
  const local = localQuery.data ?? [];
  const activeProvider = settings.providers.find(
    (p) => p.id === settings.active_provider_id,
  );
  const cpuCores = hardware.data?.cpuCores;

  const startDownload = (m: EmbeddedModel) => {
    setDownloads((d) => ({
      ...d,
      [m.id]: { downloaded: 0, total: m.sizeBytes },
    }));
    downloadModel.mutate(m.id, {
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

  // Reproduces ModelsView.activate() exactly: build/reuse a kind:"embedded"
  // provider and make it active. Instant action → update + saveNow.
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
    window.setTimeout(() => setActivating(null), 300);
  };

  return (
    <div>
      <StepHeading title={t("local.title")} description={t("local.description")} />

      {cpuCores != null && (
        <p className="mb-4 text-sm text-muted-foreground leading-relaxed">
          {cpuCores >= 4
            ? t("local.hardwareNote.ok")
            : t("local.hardwareNote.limited", { cores: cpuCores })}
        </p>
      )}

      <div className="space-y-3">
        {catalog?.map((m) => {
          const isLocal = local.includes(m.id);
          const isActive =
            activeProvider?.kind === "embedded" && activeProvider.model === m.id;
          const download = downloads[m.id];
          const downloading = !!download && !download.error;
          const failed = !!download?.error;
          const pct =
            download && download.total > 0
              ? Math.min(100, Math.round((download.downloaded / download.total) * 100))
              : 0;

          return (
            <Card key={m.id} className={cn(isActive && "border-success/50")}>
              <CardContent className="space-y-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold">{m.label}</span>
                      {m.id === RECOMMENDED_ID && (
                        <Badge variant="accent">
                          <Star className="size-3" />
                          {t("local.recommended")}
                        </Badge>
                      )}
                      {isActive && (
                        <Badge variant="success">
                          <Check className="size-3" />
                          {t("local.active")}
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground tabular-nums">
                      {prettyLang(m.language)} ·{" "}
                      <span className="font-medium text-foreground">
                        {formatBytes(m.sizeBytes)}
                      </span>
                    </p>
                  </div>

                  <div className="shrink-0">
                    {!isLocal && !downloading && (
                      <Button
                        size="sm"
                        variant="primary"
                        title={download?.error}
                        onClick={() => startDownload(m)}
                      >
                        <Download className="size-4" />
                        {failed ? t("local.retry") : t("local.download")}
                      </Button>
                    )}
                    {isLocal && isActive && (
                      <Button size="sm" variant="secondary" disabled>
                        <Check className="size-4" />
                        {t("local.active")}
                      </Button>
                    )}
                    {isLocal && !isActive && (
                      <Button
                        size="sm"
                        variant="primary"
                        disabled={activating === m.id}
                        onClick={() => activate(m)}
                      >
                        {t("local.activate")}
                      </Button>
                    )}
                  </div>
                </div>

                {downloading && (
                  <div className="space-y-1.5">
                    <Progress value={pct} />
                    <p className="text-xs text-muted-foreground tabular-nums">
                      {t("local.downloading")} {pct}%
                    </p>
                  </div>
                )}
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

/** Removes a key from a map without mutating it. Mirrors ModelsView's helper. */
function without<T>(map: Record<string, T>, key: string): Record<string, T> {
  if (!(key in map)) return map;
  const next = { ...map };
  delete next[key];
  return next;
}

function RemoteStep() {
  const { t } = useTranslation("onboarding");
  const settings = useSettingsStore((s) => s.settings)!;
  const update = useSettingsStore((s) => s.update);
  const updateActiveProvider = useSettingsStore((s) => s.updateActiveProvider);
  const presetsQuery = useProviderPresets();
  const [show, setShow] = useState(false);

  // Settings::default() already seeds a "groq" provider + activates it, so this
  // is a no-op in the common case. Guards the edge case where it was removed
  // (deleted, or the wizard was skipped and reopened): adds it back from the
  // preset, then makes it active, before the user can paste a key into it.
  useEffect(() => {
    if (!settings.providers.some((p) => p.id === "groq")) {
      const preset = presetsQuery.data?.find((p) => p.id === "groq");
      if (preset) {
        update((s) => ({
          ...s,
          providers: [...s.providers, preset],
          active_provider_id: "groq",
        }));
      }
      return;
    }
    if (settings.active_provider_id !== "groq") {
      update((s) => ({ ...s, active_provider_id: "groq" }));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [presetsQuery.data]);

  const active = settings.providers.find(
    (p) => p.id === settings.active_provider_id,
  );
  const apiKey = active?.id === "groq" ? (active.api_key ?? "") : "";

  return (
    <div>
      <StepHeading title={t("remote.title")} description={t("remote.description")} />

      <Card className="mb-4">
        <CardContent className="space-y-3">
          <p className="text-sm font-medium text-foreground">
            {t("remote.stepsTitle")}
          </p>
          <ol className="list-decimal space-y-1 pl-5 text-sm text-muted-foreground leading-relaxed">
            <li>{t("remote.step1")}</li>
            <li>{t("remote.step2")}</li>
            <li>{t("remote.step3")}</li>
          </ol>
          <a
            href="https://wren.rafaelvieiras.com/guides/groq-api-key"
            target="_blank"
            rel="noreferrer"
            className="inline-block text-sm font-medium text-primary hover:underline"
          >
            {t("remote.openConsole")}
          </a>
        </CardContent>
      </Card>

      <Field
        label={t("remote.apiKey.label")}
        htmlFor="onboarding-apikey"
        hint={t("remote.apiKey.hint")}
      >
        <div className="relative">
          <Input
            id="onboarding-apikey"
            type={show ? "text" : "password"}
            value={apiKey}
            spellCheck={false}
            autoComplete="off"
            placeholder={t("remote.apiKey.placeholder")}
            className="pr-9"
            onChange={(e) => updateActiveProvider({ api_key: e.target.value || null })}
          />
          <button
            type="button"
            onClick={() => setShow((s) => !s)}
            aria-label={show ? t("remote.apiKey.hide") : t("remote.apiKey.show")}
            className="absolute right-1 top-1/2 -translate-y-1/2 flex size-7 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            {show ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
          </button>
        </div>
      </Field>
    </div>
  );
}

function ShortcutStep() {
  const { t } = useTranslation("onboarding");
  const settings = useSettingsStore((s) => s.settings)!;
  const setField = useSettingsStore((s) => s.setField);
  return (
    <div>
      <StepHeading title={t("shortcut.title")} description={t("shortcut.description")} />
      <Card>
        <CardContent>
          <ShortcutRecorder
            value={settings.shortcut}
            onChange={(v) => setField("shortcut", v)}
          />
        </CardContent>
      </Card>
    </div>
  );
}

function TestStep() {
  const { t } = useTranslation("onboarding");
  const [value, setValue] = useState("");
  const phrase = useMemo(() => {
    const phrases = t("test.phrases", { returnObjects: true }) as string[];
    return phrases[Math.floor(Math.random() * phrases.length)];
  }, [t]);

  return (
    <div>
      <StepHeading title={t("test.title")} description={t("test.description")} />
      <Card>
        <CardContent className="space-y-3">
          <p className="rounded-sm bg-surface-2 px-3 py-2 text-sm font-medium text-foreground">
            {phrase}
          </p>
          <textarea
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder={t("test.placeholder")}
            rows={3}
            className={cn(
              "flex w-full rounded-sm border border-input bg-surface-2 px-3 py-2 text-sm text-foreground shadow-sm transition-colors",
              "placeholder:text-subtle-foreground",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-ring",
            )}
          />
          {value.length > 0 && (
            <p className="flex items-center gap-1.5 text-sm text-success animate-fade-in">
              <Check className="size-4" />
              {t("test.success")}
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function DoneStep({ onFinish }: { onFinish: () => void }) {
  const { t } = useTranslation("onboarding");
  return (
    <Card>
      <CardContent className="space-y-6 py-10 text-center">
        <h1 className="text-2xl font-semibold tracking-tight">{t("done.title")}</h1>
        <p className="text-sm text-muted-foreground leading-relaxed">
          {t("done.description")}
        </p>
        <Button variant="primary" size="lg" onClick={onFinish} className="mx-auto">
          {t("done.cta")}
        </Button>
      </CardContent>
    </Card>
  );
}
