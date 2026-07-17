import { Info, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SectionHeader } from "@/components/SectionHeader";
import { Field } from "@/components/Field";
import { SettingRow } from "@/components/SettingRow";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useInputDevices } from "@/lib/queries";
import { useSettingsStore } from "@/lib/store";

/** Default threshold (ms) when re-enabling compression — mirrors the domain. */
const DEFAULT_COMPRESS_PAUSES_MS = 2000;

/** Sentinel value for the "System default" item (Radix Select rejects ""). */
const DEVICE_SYSTEM_DEFAULT = "__wren_system_default__";

/** Subtle (i) with a Tooltip hint — replaces the old tiny ⓘ marks. */
function InfoHint({ label }: { label: string }) {
  const { t } = useTranslation("audio");
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={t("infoHint.ariaLabel")}
          className="inline-flex size-4 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Info className="size-3.5" />
        </button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

/**
 * Audio: microphone, pause compression, and feedback sounds. Follows the
 * ShortcutsView pattern (SectionHeader + Card + Field/SettingRow) and uses
 * autosave — it just calls the store setters.
 */
export default function AudioView() {
  const { t } = useTranslation("audio");
  const settings = useSettingsStore((s) => s.settings);
  const setField = useSettingsStore((s) => s.setField);
  const { data: devices, refetch, isFetching } = useInputDevices();

  if (!settings) return null;

  const inputDevices = devices ?? [];
  const compressOn = settings.compress_pauses_over_ms !== null;
  const seconds =
    (settings.compress_pauses_over_ms ?? DEFAULT_COMPRESS_PAUSES_MS) / 1000;
  // Keep the saved device even if it dropped off the list (disconnected now).
  const savedMissing =
    !!settings.input_device && !inputDevices.includes(settings.input_device);

  return (
    <div className="max-w-[560px]">
      <SectionHeader
        title={t("title")}
        description={t("description")}
      />

      <Card>
        <CardContent>
          <Field
            label={t("device.label")}
            hint={t("device.hint")}
          >
            <div className="flex items-center gap-2">
              <Select
                value={settings.input_device ?? DEVICE_SYSTEM_DEFAULT}
                onValueChange={(v) =>
                  setField(
                    "input_device",
                    v === DEVICE_SYSTEM_DEFAULT ? null : v,
                  )
                }
              >
                <SelectTrigger className="flex-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={DEVICE_SYSTEM_DEFAULT}>
                    {t("device.systemDefault")}
                  </SelectItem>
                  {savedMissing && settings.input_device && (
                    <SelectItem value={settings.input_device}>
                      {settings.input_device}
                    </SelectItem>
                  )}
                  {inputDevices.map((d) => (
                    <SelectItem key={d} value={d}>
                      {d}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={() => refetch()}
                disabled={isFetching}
                title={t("device.refresh")}
                aria-label={t("device.refresh")}
              >
                <RefreshCw className={isFetching ? "animate-spin" : undefined} />
              </Button>
            </div>
          </Field>

          <Separator className="my-4" />

          <SettingRow
            label={
              <span className="inline-flex items-center gap-1.5">
                {t("compression.label")}
                <InfoHint label={t("compression.info")} />
              </span>
            }
            hint={t("compression.hint")}
            control={
              <Switch
                checked={compressOn}
                onCheckedChange={(on) =>
                  setField(
                    "compress_pauses_over_ms",
                    on ? DEFAULT_COMPRESS_PAUSES_MS : null,
                  )
                }
              />
            }
          />

          {compressOn && (
            <div className="animate-fade-in flex items-center gap-2 pl-0 pb-1 text-sm text-foreground">
              <span>{t("compression.prefix")}</span>
              <Input
                type="number"
                min={1}
                step={1}
                aria-label={t("compression.secondsAriaLabel")}
                className="h-8 w-16 text-center"
                value={seconds}
                onChange={(e) => {
                  const secs = Number(e.target.value);
                  if (Number.isFinite(secs) && secs > 0) {
                    setField(
                      "compress_pauses_over_ms",
                      Math.round(secs * 1000),
                    );
                  }
                }}
              />
              <span>{t("compression.secondsSuffix")}</span>
            </div>
          )}

          <Separator className="my-4" />

          <SettingRow
            label={t("sounds.label")}
            hint={t("sounds.hint")}
            control={
              <Switch
                checked={settings.play_sounds}
                onCheckedChange={(on) => setField("play_sounds", on)}
              />
            }
          />
        </CardContent>
      </Card>
    </div>
  );
}
