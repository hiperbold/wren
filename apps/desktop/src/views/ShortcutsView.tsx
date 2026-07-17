import { useTranslation } from "react-i18next";
import { SectionHeader } from "@/components/SectionHeader";
import { Field } from "@/components/Field";
import { ShortcutRecorder } from "@/components/ShortcutRecorder";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettingsStore } from "@/lib/store";
import type { ActivationMode } from "@/lib/tauri";

/**
 * REFERENCE VIEW (Shortcuts). The pattern the other views should copy:
 * SectionHeader + a grouping Card + Field/ShortcutRecorder, and autosave via the
 * store (just call the setter — the save happens on its own, with "Saved"
 * feedback). UI strings live in the `shortcuts` i18n namespace.
 */
export default function ShortcutsView() {
  const { t } = useTranslation("shortcuts");
  const settings = useSettingsStore((s) => s.settings);
  const setField = useSettingsStore((s) => s.setField);

  if (!settings) return null;

  return (
    <div className="max-w-[560px]">
      <SectionHeader title={t("title")} description={t("description")} />

      <Card>
        <CardContent className="py-1">
          <Field label={t("global.label")} hint={t("global.hint")}>
            <ShortcutRecorder
              value={settings.shortcut}
              onChange={(shortcut) => setField("shortcut", shortcut)}
            />
          </Field>

          <Separator className="my-4" />

          <Field
            label={t("activation.label")}
            hint={
              settings.activation_mode === "toggle"
                ? t("activation.hint.toggle")
                : t("activation.hint.pushToTalk")
            }
          >
            <Select
              value={settings.activation_mode}
              onValueChange={(v) =>
                setField("activation_mode", v as ActivationMode)
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="toggle">
                  {t("activation.option.toggle")}
                </SelectItem>
                <SelectItem value="push_to_talk">
                  {t("activation.option.pushToTalk")}
                </SelectItem>
              </SelectContent>
            </Select>
          </Field>

          <Separator className="my-4" />

          <Field label={t("cancel.label")} hint={t("cancel.hint")}>
            <ShortcutRecorder
              value={settings.cancel_shortcut}
              onChange={(cancel_shortcut) =>
                setField("cancel_shortcut", cancel_shortcut)
              }
            />
          </Field>
        </CardContent>
      </Card>
    </div>
  );
}
