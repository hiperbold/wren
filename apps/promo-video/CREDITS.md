# Credits — Wren Promotional Video

## Formats

The same timeline generates two formats (responsive composition by orientation in
`src/scenes.tsx`):
- **Vertical 9:16 (1080×1920)** — social feed (X, LinkedIn, Product Hunt, Reddit).
  `npm run render` → `out/wren-promo.mp4`.
- **Horizontal 16:9 (1920×1080)** — YouTube, landing, wide preview.
  `npm run render:wide` → `out/wren-promo-wide.mp4`.

In landscape, the multi-app demo reflows (window on left, bubble on right); the rest
of the scenes center and adapt automatically.

## Voiceover (Active)

One voice per app, "the person dictating" text in each window — **"Jessica - Playful,
Bright, Warm"** via **ElevenLabs** (TTS), natural/relaxed delivery (`speed 0.9`):
- `public/vo/dictation-editor.mp3` — "Ship the release notes before the standup."
- `public/vo/dictation-browser.mp3` — "Flights to Lisbon in September."
- `public/vo/dictation-chat-sarah.mp3` — "On it, pushing the fix now. Tests are green."
  → This window uses **"Sarah - Confident"** (tonal comparison). Jessica version
  archived in `dictation-chat.mp3`.

To change voice, regenerate via ElevenLabs MCP with another `voice_id` and re-render.

⚠️ **CRITICAL LICENSE NOTICE:** Generated on an **ElevenLabs free-tier account** — strictly
speaking, requires attribution and **does NOT grant commercial use rights**. This is a proof of
concept; for publishing, migrate to a paid plan (or use voiceover with commercial rights).

## Music (Active) — Custom Ambient Bed

`public/music/ambient.wav` — **synthesized** by `scripts/gen-audio.mjs` (own work,
**no attribution required**). It ducks under the voiceover and is the only track the
render actually uses. Kevin MacLeod tracks downloaded during earlier iterations
(Carefree / Cheery Monday / Happy Alley / Long Note Two) are unused and **not
version-controlled** — see `.gitignore`.
Music generation via ElevenLabs requires a paid plan (blocked on free tier: `402 paid_plan_required`).

### Alternative Candidates (Same CC BY 4.0 License, Kevin MacLeod)

Downloaded for audition, not committed; change the `src` of `<Audio>` in
`src/PromoVideo.tsx` to test one. **If you adopt one, add its attribution here** —
CC BY 4.0 requires it.

- **"Carefree"** → `public/music/carefree-kevin-macleod.mp3` (cozy/upbeat — used in an earlier version)
- **"Cheery Monday"** → `public/music/cheery-monday-kevin-macleod.mp3` (more upbeat/playful)
- **"Happy Alley"** → `public/music/happy-alley-kevin-macleod.mp3` (bouncy, cozy)
- **"Long Note Two"** → `public/music/long-note-two-kevin-macleod.mp3` (ambient/drone)

> None of the Kevin MacLeod tracks are version-controlled in git — only the synthesized
> `ambient.wav` is. Download any of them to audition with:
> `curl -L "https://incompetech.com/music/royalty-free/mp3-royaltyfree/<Name>.mp3" -o public/music/<file>.mp3`

## Icons

**Lucide** (https://lucide.dev) — **ISC** license. Icons used: keyboard, mic, zap,
shield-check, feather, check. Inlined as SVG in `src/components/Icon.tsx`.

## Sound Effects (SFX)

Procedurally synthesized by `scripts/gen-audio.mjs` (click, ding, whoosh) —
**own work, no attribution required.** Files in `public/sfx/`.
The synthetic ambient bed `public/music/ambient.wav` is also own work.

## Branding

Wren wordmark and mark (`public/wren-*.png`) — project assets.
