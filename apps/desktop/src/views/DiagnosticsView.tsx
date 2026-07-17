import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ClipboardCopy, RefreshCw, Trash2 } from "lucide-react";
import { SectionHeader } from "@/components/SectionHeader";
import { OutcomeBadge } from "@/components/StatusBadge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  useClearLogs,
  useLogPath,
  useLogs,
  useMetrics,
} from "@/lib/queries";
import {
  formatLogTime,
  formatMB,
  formatWhen,
  STAGE_BG,
  STAGE_NAMES,
  stageLabel,
} from "@/lib/format";
import type { LogRecord, SessionMetrics, StageName } from "@/lib/tauri";
import { cn } from "@/lib/utils";

/** Level Select sentinel (Radix does not accept value=""). */
const LEVEL_ALL = "all";
const LEVELS = ["ERROR", "WARN", "INFO", "DEBUG"] as const;

/** Level label color (ERROR=danger, WARN=warning, INFO=info, DEBUG=muted). */
const LEVEL_COLOR: Record<string, string> = {
  ERROR: "text-danger",
  WARN: "text-warning",
  INFO: "text-info",
  DEBUG: "text-muted-foreground",
};

/** Subtle background tint for the higher-severity rows. */
const LEVEL_ROW: Record<string, string> = {
  ERROR: "bg-danger-bg",
  WARN: "bg-warning-bg",
};

/* -------------------------------- Logs ----------------------------------- */

function LogRow({ r, zebra }: { r: LogRecord; zebra: boolean }) {
  const level = r.level.toUpperCase();
  return (
    <div
      className={cn(
        "flex items-start gap-3 px-3 py-1 font-mono text-xs leading-relaxed",
        LEVEL_ROW[level] ?? (zebra ? "bg-surface/40" : undefined),
      )}
    >
      <span className="w-[68px] shrink-0 tabular-nums text-muted-foreground">
        {formatLogTime(r.ts_ms)}
      </span>
      <span
        className={cn(
          "w-12 shrink-0 font-semibold tabular-nums",
          LEVEL_COLOR[level] ?? "text-muted-foreground",
        )}
      >
        {level}
      </span>
      <span
        className="w-36 shrink-0 truncate text-subtle-foreground"
        title={r.target}
      >
        {r.target}
      </span>
      <span className="min-w-0 flex-1 whitespace-pre-wrap break-words text-foreground">
        {r.message}
      </span>
    </div>
  );
}

function LogsPanel() {
  const { t } = useTranslation("diagnostics");
  const [query, setQuery] = useState("");
  const [level, setLevel] = useState<string>(LEVEL_ALL);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const logsQuery = useLogs(
    level === LEVEL_ALL ? null : level,
    query || null,
    autoRefresh,
  );
  const logPath = useLogPath();
  const clearLogs = useClearLogs();

  const logs = logsQuery.data ?? [];

  const copyPath = () => {
    if (logPath.data) navigator.clipboard?.writeText(logPath.data).catch(() => {});
  };

  return (
    <div className="space-y-4">
      {/* Aligned filter bar. Never two accent buttons side by side:
          Refresh is ghost and Clear is destructive (with confirmation). */}
      <div className="flex flex-wrap items-center gap-2">
        <Input
          type="search"
          placeholder={t("search.placeholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="h-9 min-w-[200px] flex-1"
        />
        <Select value={level} onValueChange={setLevel}>
          <SelectTrigger className="h-9 w-[150px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={LEVEL_ALL}>{t("level.all")}</SelectItem>
            {LEVELS.map((lv) => (
              <SelectItem key={lv} value={lv}>
                {lv}+
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <div className="flex items-center gap-2 pl-1">
          <Switch
            id="log-auto"
            checked={autoRefresh}
            onCheckedChange={setAutoRefresh}
          />
          <Label htmlFor="log-auto" className="cursor-pointer text-xs">
            {t("autoRefresh.label")}
          </Label>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => logsQuery.refetch()}
          title={t("logs.refreshTooltip")}
        >
          <RefreshCw
            className={logsQuery.isFetching ? "animate-spin" : undefined}
          />
          {t("refresh.label")}
        </Button>
        <Dialog>
          <DialogTrigger asChild>
            <Button variant="destructive" size="sm">
              <Trash2 />
              {t("clear.label")}
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t("clear.title")}</DialogTitle>
              <DialogDescription>
                {t("clear.description")}
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <DialogClose asChild>
                <Button variant="secondary" size="sm">
                  {t("clear.cancel")}
                </Button>
              </DialogClose>
              <DialogClose asChild>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={() => clearLogs.mutate()}
                >
                  <Trash2 />
                  {t("clear.confirm")}
                </Button>
              </DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      <div className="overflow-hidden rounded-lg border border-border bg-card">
        {logs.length === 0 ? (
          <p className="px-3 py-10 text-center text-sm text-muted-foreground">
            {logsQuery.isLoading
              ? t("loading.logs")
              : t("empty.logs")}
          </p>
        ) : (
          <ScrollArea className="h-[420px]">
            <div className="py-1">
              {logs.map((r, i) => (
                <LogRow key={i} r={r} zebra={i % 2 === 1} />
              ))}
            </div>
          </ScrollArea>
        )}
      </div>

      <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
        <span>{t("file.label")}</span>
        <code className="rounded-sm bg-surface-2 px-1.5 py-0.5 font-mono text-xs text-foreground">
          {logPath.data ?? "…"}
        </code>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2"
          onClick={copyPath}
          disabled={!logPath.data}
        >
          <ClipboardCopy />
          {t("file.copy")}
        </Button>
      </p>
    </div>
  );
}

/* ----------------------------- Telemetry --------------------------------- */

/** Single stage legend, at the top (not repeated in each card). */
function StageLegend() {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
      {STAGE_NAMES.map((s) => (
        <span
          key={s}
          className="flex items-center gap-1.5 text-xs text-muted-foreground"
        >
          <i className={cn("size-2.5 rounded-[3px]", STAGE_BG[s])} />
          {stageLabel(s)}
        </span>
      ))}
    </div>
  );
}

function MetricCard({ m }: { m: SessionMetrics }) {
  // Slowest stage: highlighted in the numbers (the color legend lives at the top).
  const slowest: StageName | null = m.stages.length
    ? m.stages.reduce((a, b) => (a.duration_ms >= b.duration_ms ? a : b)).stage
    : null;

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="mb-3 flex flex-wrap items-center gap-x-3 gap-y-1">
        <OutcomeBadge outcome={m.outcome} />
        <span className="text-xs text-muted-foreground tabular-nums">
          {formatWhen(m.created_at_ms)}
        </span>
        {m.provider_id && (
          <span className="text-xs text-muted-foreground">{m.provider_id}</span>
        )}
        <span className="ml-auto text-sm font-semibold tabular-nums text-foreground">
          {m.total_ms} ms
        </span>
        <span className="text-xs text-muted-foreground tabular-nums">
          RSS {formatMB(m.rss_peak_bytes)}
        </span>
      </div>

      {m.stages.length > 0 && (
        <div className="flex h-2.5 overflow-hidden rounded-full bg-surface-2">
          {m.stages.map((s, j) => (
            <span
              key={j}
              className={STAGE_BG[s.stage]}
              style={{
                width: `${m.total_ms ? (s.duration_ms / m.total_ms) * 100 : 0}%`,
              }}
              title={`${stageLabel(s.stage)}: ${s.duration_ms} ms`}
            />
          ))}
        </div>
      )}

      {m.stages.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-muted-foreground tabular-nums">
          {m.stages.map((s, j) => (
            <span
              key={j}
              className={cn(
                s.stage === slowest && "font-semibold text-foreground",
              )}
            >
              {stageLabel(s.stage)} {s.duration_ms}ms
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function TelemetryPanel() {
  const { t } = useTranslation("diagnostics");
  const metricsQuery = useMetrics();
  const metrics = useMemo(() => metricsQuery.data ?? [], [metricsQuery.data]);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => metricsQuery.refetch()}
          title={t("telemetry.refreshTooltip")}
        >
          <RefreshCw
            className={metricsQuery.isFetching ? "animate-spin" : undefined}
          />
          {t("refresh.label")}
        </Button>
        <span className="text-xs text-muted-foreground">
          {t("telemetry.summary", { count: metrics.length })}
        </span>
      </div>

      {metrics.length === 0 ? (
        <p className="rounded-lg border border-dashed border-border bg-surface/40 px-6 py-12 text-center text-sm text-muted-foreground">
          {metricsQuery.isLoading
            ? t("telemetry.loading")
            : t("telemetry.empty")}
        </p>
      ) : (
        <>
          {/* Single legend at the top, instead of repeating the legend in each card. */}
          <div className="rounded-lg border border-border bg-surface/40 px-4 py-2.5">
            <StageLegend />
          </div>
          <div className="space-y-3">
            {metrics.map((m, i) => (
              <MetricCard key={m.created_at_ms || i} m={m} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

// Diagnostics: Logs (searchable, auto-refresh) and Telemetry (per-session stage
// bars) in sub-navigation with the shadcn <Tabs/>. Everything is 100% local.
export default function DiagnosticsView() {
  const { t } = useTranslation("diagnostics");
  return (
    <div className="max-w-[760px]">
      <SectionHeader
        title={t("title")}
        description={t("description")}
      />

      <Tabs defaultValue="logs">
        <TabsList>
          <TabsTrigger value="logs">{t("tab.logs")}</TabsTrigger>
          <TabsTrigger value="telemetria">{t("tab.telemetry")}</TabsTrigger>
        </TabsList>
        <TabsContent value="logs">
          <LogsPanel />
        </TabsContent>
        <TabsContent value="telemetria">
          <TelemetryPanel />
        </TabsContent>
      </Tabs>
    </div>
  );
}
