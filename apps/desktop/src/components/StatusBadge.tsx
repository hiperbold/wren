import { useTranslation } from "react-i18next";
import { Cloud, Lock } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { outcomeLabel } from "@/lib/format";
import type { SessionOutcome } from "@/lib/tauri";

/** SINGLE map of session outcome → status variant. Use in History AND Telemetry
 * (the audit asked for consistent statuses across screens). */
const OUTCOME_VARIANT: Record<
  SessionOutcome,
  "success" | "warning" | "danger" | "neutral"
> = {
  delivered: "success",
  discarded_no_speech: "neutral",
  failed: "danger",
  cancelled: "warning",
};

/** Badge for a session outcome (Telemetry). */
export function OutcomeBadge({ outcome }: { outcome: SessionOutcome }) {
  return (
    <Badge variant={OUTCOME_VARIANT[outcome]}>{outcomeLabel(outcome)}</Badge>
  );
}

/** Status badge for a History item. */
export function HistoryStatusBadge({ status }: { status: "done" | "failed" }) {
  const { t } = useTranslation("common");
  return status === "failed" ? (
    <Badge variant="danger">{t("badge.failed")}</Badge>
  ) : (
    <Badge variant="success">{t("badge.ok")}</Badge>
  );
}

/** Egress badge (does the audio leave the machine?). Neutral/local vs.
 * warning/external, anchored next to the provider name. */
export function EgressBadge({ external }: { external: boolean }) {
  const { t } = useTranslation("common");
  return external ? (
    <Badge variant="warning" title={t("badge.externalTitle")}>
      <Cloud className="size-3" />
      {t("badge.external")}
    </Badge>
  ) : (
    <Badge variant="success" title={t("badge.localTitle")}>
      <Lock className="size-3" />
      {t("badge.local")}
    </Badge>
  );
}
