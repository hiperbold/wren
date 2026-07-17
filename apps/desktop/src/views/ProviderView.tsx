import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import {
  Check,
  ClipboardPaste,
  Copy,
  Eye,
  EyeOff,
  Loader2,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { SectionHeader } from "@/components/SectionHeader";
import { Field } from "@/components/Field";
import { EgressBadge } from "@/components/StatusBadge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useSettingsStore } from "@/lib/store";
import { useModels, useProviderPresets } from "@/lib/queries";
import { egressIsExternal, uniqueProviderId } from "@/lib/format";
import type { ProviderConfig } from "@/lib/tauri";

/** Sentinel for the "Other… (type it)" option in the model selector. */
const MODEL_OTHER = "__wren_outro__";
const LANGUAGE_AUTO = "__wren_auto__";

/** Common languages (ISO 639-1 code stored in `settings.language`). */
const LANGUAGES: { code: string; label: string }[] = [
  { code: "pt", label: "Português" },
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "it", label: "Italiano" },
];

/**
 * VIEW: Provider (the transcription service). Active-provider selection, the
 * address/key/model/language fields, an egress badge anchored to the name, an
 * implicit connection test (if it lists models, it connected) and a "Manage"
 * sub-section with add-from-preset / duplicate / remove. AUTOSAVE via the store.
 */
export default function ProviderView() {
  const { t } = useTranslation("provider");
  const settings = useSettingsStore((s) => s.settings);
  const setField = useSettingsStore((s) => s.setField);
  const updateActiveProvider = useSettingsStore((s) => s.updateActiveProvider);
  const update = useSettingsStore((s) => s.update);

  const presetsQuery = useProviderPresets();

  if (!settings) return null;

  const active = settings.providers.find(
    (p) => p.id === settings.active_provider_id,
  );
  const isEmbedded = active?.kind === "embedded";
  const external = active ? egressIsExternal(active.base_url) : false;
  const presets = presetsQuery.data ?? [];

  // Adds a copy of a preset (address + model already filled in) and makes it
  // the active provider — the user only pastes the key. Mirrors addFromPreset.
  const addFromPreset = (presetId: string) => {
    const preset = presets.find((p) => p.id === presetId);
    if (!preset) return;
    const id = uniqueProviderId(preset.id, settings.providers);
    const copy: ProviderConfig = { ...preset, id };
    update((s) => ({
      ...s,
      providers: [...s.providers, copy],
      active_provider_id: id,
    }));
  };

  // Duplicates the active provider (new id, "… (copy)") and activates the copy.
  const duplicateActive = () => {
    if (!active) return;
    const id = uniqueProviderId(active.id, settings.providers);
    const copy: ProviderConfig = {
      ...active,
      id,
      label: `${active.label}${t("copySuffix")}`,
    };
    update((s) => ({
      ...s,
      providers: [...s.providers, copy],
      active_provider_id: id,
    }));
  };

  // Removes the active provider; activates the first of the remaining ones.
  const removeActive = () => {
    if (settings.providers.length <= 1) return;
    update((s) => {
      const remaining = s.providers.filter(
        (p) => p.id !== s.active_provider_id,
      );
      return {
        ...s,
        providers: remaining,
        active_provider_id: remaining[0].id,
      };
    });
  };

  return (
    <div className="max-w-[560px]">
      <SectionHeader title={t("title")} description={t("description")} />

      <Card>
        <CardContent className="space-y-4">
          <Field
            label={t("active.label")}
            htmlFor="provider-active"
            labelAddon={
              active ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span tabIndex={0} className="inline-flex cursor-help">
                      <EgressBadge external={external} />
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>
                    {external
                      ? t("egress.tooltip.external")
                      : t("egress.tooltip.local")}
                  </TooltipContent>
                </Tooltip>
              ) : undefined
            }
          >
            <Select
              value={settings.active_provider_id}
              onValueChange={(id) => setField("active_provider_id", id)}
            >
              <SelectTrigger id="provider-active">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {settings.providers.map((p) => (
                  <SelectItem key={p.id} value={p.id}>
                    {p.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {active && isEmbedded && (
            <>
              <Separator />
              <p className="text-sm text-muted-foreground leading-relaxed">
                <Trans
                  t={t}
                  i18nKey="embedded.note"
                  components={{
                    1: <span className="font-medium text-foreground" />,
                  }}
                />
              </p>
            </>
          )}

          {active && !isEmbedded && (
            <ProviderFields
              key={active.id}
              active={active}
              external={external}
              onPatch={updateActiveProvider}
            />
          )}

          <Separator />

          <Field
            label={t("language.label")}
            htmlFor="provider-language"
            hint={t("language.hint")}
          >
            <Select
              value={settings.language ?? LANGUAGE_AUTO}
              onValueChange={(v) =>
                setField("language", v === LANGUAGE_AUTO ? null : v)
              }
            >
              <SelectTrigger id="provider-language">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={LANGUAGE_AUTO}>
                  {t("language.auto")}
                </SelectItem>
                {LANGUAGES.map((lang) => (
                  <SelectItem key={lang.code} value={lang.code}>
                    {lang.label}
                  </SelectItem>
                ))}
                {settings.language &&
                  !LANGUAGES.some((l) => l.code === settings.language) && (
                    <SelectItem value={settings.language}>
                      {settings.language}
                    </SelectItem>
                  )}
              </SelectContent>
            </Select>
          </Field>
        </CardContent>
      </Card>

      <ManageProviders
        presets={presets}
        activeLabel={active?.label ?? ""}
        canRename={!!active && !isEmbedded}
        canRemove={settings.providers.length > 1}
        onRename={(label) => updateActiveProvider({ label })}
        onAddPreset={addFromPreset}
        onDuplicate={duplicateActive}
        onRemove={removeActive}
      />
    </div>
  );
}

/* ---------------------------- provider fields --------------------------- */

/** Address / API key / model fields for the active (non-embedded) provider. The
 * order is adaptive: cloud prioritizes the key; local prioritizes the address. */
function ProviderFields({
  active,
  external,
  onPatch,
}: {
  active: ProviderConfig;
  external: boolean;
  onPatch: (patch: Partial<ProviderConfig>) => void;
}) {
  const { t } = useTranslation("provider");
  // A single models fetch: feeds both the dropdown AND the connection indicator.
  const models = useModels(active.base_url, active.api_key);

  const endpointField = (
    <Field
      label={external ? t("endpoint.label.external") : t("endpoint.label.local")}
      htmlFor="provider-endpoint"
      hint={
        external ? t("endpoint.hint.external") : t("endpoint.hint.local")
      }
    >
      <Input
        id="provider-endpoint"
        value={active.base_url}
        placeholder="http://localhost:8555/v1"
        spellCheck={false}
        onChange={(e) => onPatch({ base_url: e.target.value })}
      />
    </Field>
  );

  const apiKeyField = (
    <ApiKeyField
      external={external}
      value={active.api_key ?? ""}
      onChange={(v) => onPatch({ api_key: v || null })}
    />
  );

  return (
    <>
      <Separator />
      {external ? (
        <>
          {apiKeyField}
          {endpointField}
        </>
      ) : (
        <>
          {endpointField}
          {apiKeyField}
        </>
      )}

      <Field
        label={t("model.label")}
        labelAddon={<ConnectionIndicator models={models} />}
      >
        <ModelSelect
          models={models.data}
          isLoading={models.isLoading && models.fetchStatus !== "idle"}
          value={active.model}
          onChange={(model) => onPatch({ model })}
          onRefresh={() => models.refetch()}
        />
      </Field>
    </>
  );
}

/** API key field: `type=password` with show/hide (eye) and paste. */
function ApiKeyField({
  external,
  value,
  onChange,
}: {
  external: boolean;
  value: string;
  onChange: (v: string) => void;
}) {
  const { t } = useTranslation("provider");
  const [show, setShow] = useState(false);

  const paste = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) onChange(text.trim());
    } catch {
      /* clipboard unavailable — no action */
    }
  };

  return (
    <Field
      label={external ? t("apiKey.label.external") : t("apiKey.label.local")}
      htmlFor="provider-apikey"
      hint={external ? t("apiKey.hint.external") : t("apiKey.hint.local")}
    >
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Input
            id="provider-apikey"
            type={show ? "text" : "password"}
            value={value}
            spellCheck={false}
            autoComplete="off"
            placeholder={
              external
                ? t("apiKey.placeholder.external")
                : t("apiKey.placeholder.local")
            }
            className="pr-9"
            onChange={(e) => onChange(e.target.value)}
          />
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => setShow((s) => !s)}
                aria-label={show ? t("apiKey.hide") : t("apiKey.show")}
                className="absolute right-1 top-1/2 -translate-y-1/2 flex size-7 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                {show ? (
                  <EyeOff className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent>
              {show ? t("apiKey.hide") : t("apiKey.show")}
            </TooltipContent>
          </Tooltip>
        </div>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="outline"
              size="icon"
              onClick={paste}
              aria-label={t("apiKey.paste.ariaLabel")}
            >
              <ClipboardPaste className="size-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("apiKey.paste.tooltip")}</TooltipContent>
        </Tooltip>
      </div>
    </Field>
  );
}

/** Connection indicator: derived from `useModels` success (if it lists models,
 * it connected). Green = connected; spinning = checking. Silent on failure (the
 * ModelSelect already falls back to free text and explains). */
function ConnectionIndicator({
  models,
}: {
  models: ReturnType<typeof useModels>;
}) {
  const { t } = useTranslation("provider");
  if (models.fetchStatus === "fetching") {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
        <Loader2 className="size-3 animate-spin" />
        {t("connection.checking")}
      </span>
    );
  }
  if (models.isSuccess && (models.data?.length ?? 0) > 0) {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-success">
        <Check className="size-3" />
        {t("connection.connected")}
      </span>
    );
  }
  return null;
}

/** Model selector with a dynamic list: dropdown when the server lists them
 * (`useModels`), with an "Other…" option and a fallback to free text when the
 * list fails (local servers that don't expose /models). Reload button. */
function ModelSelect({
  models,
  isLoading,
  value,
  onChange,
  onRefresh,
}: {
  models: string[] | undefined;
  isLoading: boolean;
  value: string;
  onChange: (m: string) => void;
  onRefresh: () => void;
}) {
  const { t } = useTranslation("provider");
  const [typingOther, setTypingOther] = useState(false);

  const refreshButton = (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={onRefresh}
          disabled={isLoading}
          aria-label={t("model.refresh.ariaLabel")}
        >
          <RefreshCw
            className={"size-4" + (isLoading ? " animate-spin" : "")}
          />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{t("model.refresh.tooltip")}</TooltipContent>
    </Tooltip>
  );

  const hasList = !!models && models.length > 0;

  // List available → dropdown (preserving the current value even if off-list).
  if (hasList && !typingOther) {
    const inList = value !== "" && models!.includes(value);
    const extra = value && !inList ? [value] : [];
    return (
      <div className="flex gap-2">
        <Select
          value={value || undefined}
          onValueChange={(v) => {
            if (v === MODEL_OTHER) setTypingOther(true);
            else onChange(v);
          }}
        >
          <SelectTrigger className="flex-1">
            <SelectValue placeholder={t("model.select.placeholder")} />
          </SelectTrigger>
          <SelectContent>
            {extra.map((m) => (
              <SelectItem key={m} value={m}>
                {m}
              </SelectItem>
            ))}
            {models!.map((m) => (
              <SelectItem key={m} value={m}>
                {m}
              </SelectItem>
            ))}
            <SelectItem value={MODEL_OTHER}>{t("model.other")}</SelectItem>
          </SelectContent>
        </Select>
        {refreshButton}
      </div>
    );
  }

  // No list (or "Other…") → free text.
  return (
    <div className="space-y-1.5">
      <div className="flex gap-2">
        <Input
          className="flex-1"
          value={value}
          placeholder={t("model.input.placeholder")}
          spellCheck={false}
          autoFocus={typingOther}
          onChange={(e) => onChange(e.target.value)}
        />
        {refreshButton}
      </div>
      {!hasList && !isLoading && (
        <p className="text-sm text-muted-foreground leading-relaxed">
          {t("model.listFailed")}
        </p>
      )}
    </div>
  );
}

/* ------------------------------ manage CRUD ----------------------------- */

/** Bounded sub-section (its own Card) for adding/duplicating/renaming/removing
 * providers. "Add" is a labeled preset Select (not a hybrid button-select),
 * "Remove" is destructive with confirmation. */
function ManageProviders({
  presets,
  activeLabel,
  canRename,
  canRemove,
  onRename,
  onAddPreset,
  onDuplicate,
  onRemove,
}: {
  presets: ProviderConfig[];
  activeLabel: string;
  canRename: boolean;
  canRemove: boolean;
  onRename: (label: string) => void;
  onAddPreset: (id: string) => void;
  onDuplicate: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation("provider");
  const [confirmRemove, setConfirmRemove] = useState(false);

  return (
    <Card className="mt-4 bg-surface">
      <CardContent className="space-y-4">
        <div className="space-y-0.5">
          <h2 className="text-lg font-semibold tracking-tight">
            {t("manage.title")}
          </h2>
          <p className="text-sm text-muted-foreground">
            {t("manage.description")}
          </p>
        </div>

        <Field
          label={t("manage.add.label")}
          htmlFor="provider-add"
          hint={t("manage.add.hint")}
        >
          <Select value="" onValueChange={onAddPreset}>
            <SelectTrigger id="provider-add">
              <SelectValue placeholder={t("manage.add.placeholder")} />
            </SelectTrigger>
            <SelectContent>
              {presets.map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  {p.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        {canRename && (
          <Field label={t("manage.rename.label")} htmlFor="provider-rename">
            <Input
              id="provider-rename"
              value={activeLabel}
              onChange={(e) => onRename(e.target.value)}
            />
          </Field>
        )}

        <Separator />

        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={onDuplicate}>
            <Copy className="size-4" />
            {t("action.duplicate")}
          </Button>
          <Tooltip>
            <TooltipTrigger asChild>
              <span className={canRemove ? "" : "cursor-not-allowed"}>
                <Button
                  variant="destructive"
                  size="sm"
                  disabled={!canRemove}
                  onClick={() => setConfirmRemove(true)}
                >
                  <Trash2 className="size-4" />
                  {t("action.remove")}
                </Button>
              </span>
            </TooltipTrigger>
            {!canRemove && (
              <TooltipContent>
                {t("manage.remove.disabledTooltip")}
              </TooltipContent>
            )}
          </Tooltip>
        </div>
      </CardContent>

      <Dialog open={confirmRemove} onOpenChange={setConfirmRemove}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("dialog.remove.title")}</DialogTitle>
            <DialogDescription>
              {t("dialog.remove.description", { label: activeLabel })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmRemove(false)}>
              {t("action.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                onRemove();
                setConfirmRemove(false);
              }}
            >
              <Trash2 className="size-4" />
              {t("action.remove")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
