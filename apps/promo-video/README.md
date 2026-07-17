# Wren — Promotional Video (Remotion)

Wren's marketing video, built **100% in code** with [Remotion](https://remotion.dev).
**Vertical 9:16 format** (1080×1920, 30fps, ~30s) for social media (X, LinkedIn,
Product Hunt, Reddit).

The bubble/overlay is **recreated in web** (SVG, faithful to the native shader) — **never**
screen-captured from the real UI. This keeps the visual art-directed and mockable, with no risk
to the app's native render (see `src/components/Bubble.tsx` and `src/theme.ts`, which mirror the
native overlay's constants 1:1).

## Running

```bash
npm install
npm run audio      # generates SFX + ambient bed (synthetic WAV, royalty-free)
npm run studio     # interactive preview in Remotion Studio
npm run render     # exports out/wren-promo.mp4
```

`npm run still -- --frame=300` exports a single frame (useful for thumbnail).

## Structure

| File | Purpose |
| --- | --- |
| `src/Root.tsx` | Registers the `WrenPromo` composition (dimensions, fps, duration). |
| `src/PromoVideo.tsx` | Timeline: sequence of scenes + audio track. |
| `src/scenes.tsx` | The 6 scenes (cold open, recording, processing/done, multi-app, value props, CTA). |
| `src/components/Bubble.tsx` | Web recreation of the native overlay (pill + waveform + state). |
| `src/waveform.ts` | Bar levels (deterministic "speech" envelope + shader pulse). |
| `src/theme.ts` | Palette and constants **extracted 1:1 from the app** (settings.css + WGSL shader). |
| `src/strings.ts` | **All video copy**. Switching `LANG` localizes everything. |
| `scripts/gen-audio.mjs` | Generates SFX WAVs and ambient bed. |

## Customizing

- **Language:** `src/strings.ts` → `export const LANG: Lang = "en"` (or `"pt"`).
  All text, hero voiceover, and labels change at once.
- **Text / use case:** edit fields in `src/strings.ts` (the dictated phrase,
  shown apps, value props, CTA line, and repo URL).
- **Colors:** `src/theme.ts` — kept in sync with the app. If the shader changes,
  update here too.
- **Music track:** the active track is `public/music/ambient.wav`, synthesized by
  `scripts/gen-audio.mjs` (own work, no attribution needed). A few Kevin MacLeod tracks
  (CC BY 4.0) are available for audition but not committed — see `CREDITS.md` for
  attribution requirements if you swap one in. SFX (click/ding/whoosh) are also synthetic,
  no attribution needed. To change music, point the `<Audio>` `src` in `PromoVideo.tsx` to
  another file in `public/music/` and adjust `trimBefore`/`volume`.
- **Icons:** Lucide SVG (ISC license) inlined in `src/components/Icon.tsx` — recolorable
  (amber) and scalable. Used in the hook and benefit cards.
- **Duration/cuts:** frame markers live in the `T` object in `PromoVideo.tsx`.

## Exporting in a different aspect ratio (e.g., 16:9 for README/hero)

Duplicate the `<Composition>` in `Root.tsx` with `width={1920} height={1080}` and a
new `id`. Scenes use `AbsoluteFill`/flex and re-center themselves; larger text may
need fine-tuning of `fontSize`.

> ℹ️ **CTA points to `wren.rafaelvieiras.com`** (the landing/subdomain, still under construction).
> The platform line (`ctaPlatforms`) reads "Linux today · macOS & Windows soon" — honest about
> current packaging (`tauri.conf.json` generates `deb`/`appimage` only). Update when macOS/Windows
> are packaged.
