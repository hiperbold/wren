# View guide (redesign) — read before touching anything

You'll be redesigning **one** view. Each view is **one self-contained file**
in `src/views/`. Only edit your own file (and, if you need a new sidebar
icon, `src/app/nav.ts`). Do NOT change `lib/`, `components/ui/`, `app/App.tsx`,
or the tokens — they're the shared contract. `ShortcutsView.tsx` is the
complete **reference**; copy its pattern.

## Visual pattern for a view

```tsx
import { SectionHeader } from "@/components/SectionHeader";
// ...
export default function MyView() {
  return (
    <div className="max-w-[560px]">        {/* 560 for forms; 760 for data */}
      <SectionHeader title="…" description="…" />
      <Card><CardContent>…</CardContent></Card>
    </div>
  );
}
```

- **Default export** of a prop-less component (`nav.ts` lazy-imports it).
- Width: forms `max-w-[560px]`; data screens `max-w-[760px]`.
- Group controls in a `<Card>`; separate rows with `<Separator className="my-4" />`.
- Page title = `text-xl`; section/card title = `text-lg`; body/label =
  `text-base` (14px); help/meta = `text-sm`/`text-xs`.

## Reading and writing settings (AUTOSAVE) — `@/lib/store`

There's no "Save" button. Just change the state; the store persists itself
(500ms debounce) and the App shows "Saved"/error feedback. **Never call
`save_settings`.**

```ts
import { useSettingsStore } from "@/lib/store";

const settings = useSettingsStore((s) => s.settings);          // may be null on 1st render
const setField = useSettingsStore((s) => s.setField);          // top-level field
const updateActiveProvider = useSettingsStore((s) => s.updateActiveProvider);
const update = useSettingsStore((s) => s.update);              // generic updater
const saveNow = useSettingsStore((s) => s.saveNow);            // forces an immediate save

if (!settings) return null;

setField("play_sounds", true);                                 // → autosave
updateActiveProvider({ api_key: "…" });                        // patches the active provider
update((s) => ({ ...s, providers: [...s.providers, newProvider], active_provider_id: newProvider.id }));
saveNow();                                                     // for "instant" actions (activating a model)
```

The visual feedback is already global (`<SaveIndicator/>` in App). You don't render anything for it.

## Data hooks (TanStack Query) — `@/lib/queries`

Reads (never call `invoke` directly):
`useHistory`, `useProviderPresets`, `useInputDevices`,
`useModels(baseUrl, apiKey)`, `useEmbeddedCatalog`, `useLocalModels`,
`useLogs(level, query, autoRefresh)`, `useLogPath`, `useMetrics`.
Each returns `{ data, isLoading, isError, error, refetch }`.

Writes (mutations): `useRetryTranscription`, `useCheckUpdates`,
`useDownloadModel`, `useDeleteModel`, `useClearLogs`. E.g.:
```ts
const retry = useRetryTranscription();
retry.mutate(entry.created_at_ms);            // retry.isPending for loading state
```

Download progress (Models view) comes via **event**, not a hook:
```ts
import { onDownloadProgress } from "@/lib/tauri";
useEffect(() => {
  let un: undefined | (() => void);
  onDownloadProgress((p) => { /* p.id, p.downloaded, p.total, p.done, p.error */ }).then(f => un = f);
  return () => un?.();
}, []);
```

## Available components

**Compound** (`@/components/…`):
- `Field({ label, htmlFor?, hint?, error?, labelAddon?, children })` — label+control+help with fixed spacing. Stack Fields in a `space-y-4`.
- `SettingRow({ label, hint?, control, disabled? })` — a `[label/help] —— [Switch]` row, for toggles inside a Card.
- `SectionHeader({ title, description?, actions? })` — the view's header.
- `ShortcutRecorder({ value, onChange })` and `Keycap` — shortcut recording.
- `OutcomeBadge({ outcome })`, `HistoryStatusBadge({ status })`, `EgressBadge({ external })` — semantic status indicators (single source of truth).
- `ComingSoon` — placeholder (swap for the real content).

**shadcn** (`@/components/ui/…`): `Button`, `Input`, `Label`, `Switch`,
`Select` (+ `SelectTrigger/Content/Item/Value`), `Card` (+ Header/Title/Description/Content),
`Badge`, `Tabs` (+ `TabsList/Trigger/Content`), `Dialog` (+ parts),
`Progress`, `Tooltip` (+ `TooltipTrigger/Content`; `TooltipProvider` is already in the App),
`Separator`, `ScrollArea`.

**Button** `variant`: `primary` (main action), `secondary`, `outline`,
`ghost` (row actions, e.g. Copy), `destructive` (Remove/Clear/Retry),
`link`. `size`: `sm | md | lg | icon`. Use `destructive` for anything that
deletes — never put two `primary` buttons side by side.

**Badge** `variant`: `neutral | success | warning | danger | info | accent`.

## Tokens (Tailwind, defined in `src/index.css`)

Colors (classes): `bg-background`, `bg-surface`, `bg-surface-2`, `bg-card`,
`text-foreground`, `text-muted-foreground` (help text — AA), `text-subtle-foreground`
(only ≥16px/non-essential), `border-border`, `border-border-strong`,
`bg-primary`/`text-primary` (accent — ONLY for the primary action and active state),
status `text-success|warning|danger|info` (+ `bg-*-bg`),
pipeline stages `bg-stage-capture|vad|persist|transcribe|deliver` (+ `STAGE_BG` in
`@/lib/format`).
Radii: `rounded-sm` (6, inputs/buttons/badges), `rounded-lg` (10, cards),
`rounded-xl` (14, dialogs). Focus: `focus-visible:ring-2 focus-visible:ring-ring`.

## Pure helpers — `@/lib/format`

`formatSecs, formatWhen, formatLogTime, formatMB, formatBytes, prettyLang,
prettyKey, egressIsExternal, uniqueProviderId, STAGE_NAMES, stageLabel,
STAGE_BG, outcomeLabel`. `STAGE_NAMES` is the ordered array of pipeline
stages; `stageLabel(stage)` and `outcomeLabel(outcome)` return the localized
label for a stage/outcome (there's no static label map anymore — always go
through these helpers rather than hardcoding strings). Types live in
`@/lib/tauri` (`Settings, ProviderConfig, HistoryEntry, LogRecord,
SessionMetrics, EmbeddedModel, …`).

## Text and i18n

All user-facing strings go through `react-i18next`, one namespace per view:
`const { t } = useTranslation("<namespace>")`, then `t("key")` — never
hardcode UI copy inline. Shared/cross-view labels (stage names, outcomes,
language names, etc.) live in the `common` namespace and are resolved by the
`@/lib/format` helpers above, not by calling `t()` directly in the view.

## Animation

Subtle, ≤200ms, already respects `prefers-reduced-motion` (global). Classes:
`animate-fade-in`, `animate-pop`, `animate-pulse-ring`. Don't animate data lists.

## Running it

`npm run dev:mock` → http://localhost:5173 (or the next free port). Mock mode
swaps the Tauri backend for fake data; your `invoke`/hooks work the same way.
