# Golden set — transcription quality baseline

A small, fixed set of PT-BR (Brazilian Portuguese) speech benchmark samples —
audio `.wav` files paired with reference `.txt` transcripts — used by
`wren-bench` (`scripts/bench-stt.sh`) to measure **content** (WER) and
**punctuation** (PER) quality of Wren's full pipeline: normalize → VAD (gate +
trim + pause compression) → FLAC → provider. It serves as a before/after
baseline for any pipeline change (compression artifacts, model swap,
Phase 4 LLM post-processing, etc.).

Note: the reference transcripts are, by design, in Portuguese — they are the
expected transcription of Portuguese audio, so they stay in PT-BR even in
this English-language doc.

## Structure

- `manifest.json` — lists the samples: audio, reference, scenario, and the
  `punctuation` field, which says how much to trust the reference's
  punctuation:
  - `exact` — the reference was **written before** recording; punctuation is
    ground truth (only `personal/` samples).
  - `curated` — punctuation chosen by consensus/curation across near-perfect
    transcriptions from several models (public dataset). Good, not
    infallible.
  - `none` — no relevant punctuation; the sample only counts toward WER.
- `public/` — 4 samples from the
  [tech4humans/Audio-Transcription-Models-Comparison-PT-BR](https://huggingface.co/datasets/tech4humans/Audio-Transcription-Models-Comparison-PT-BR)
  dataset (Apache-2.0): noisy reading, everyday speech, Recife accent, and a
  numeric entity. Content ground truth comes from the dataset's own WER-0
  lines; punctuation was curated (the dataset's "WER 0" entries are
  post-normalization and disagree with each other on punctuation).
- `personal/` — samples dictated by the project owner (see recording
  protocol below). These are the only ones that cover Wren's real-world
  scenario: **dictation with long thinking pauses**, which doesn't exist in
  any public dataset.

## Personal samples — how to record

Three scripts are already prepared in `personal/` (`*.roteiro.md`), with the
text to read, pause markers (⏸), and what each pause tests. The reference
transcripts (`*.txt`) and the `manifest.json` entries also already exist —
the bench skips (with a warning) any that don't have audio yet. To record:

```sh
./scripts/record-golden.sh --list           # what's left to record
./scripts/record-golden.sh ditado-prompt    # shows the script and records (Ctrl+C to stop)
./scripts/bench-stt.sh --samples ditado-prompt   # measures the newly recorded sample
```

Golden rules for recording: read the text **exactly** as written (if you slip
up, re-record); during pauses, keep genuine silence for the marked duration;
use natural question/exclamation intonation where applicable.

### To create a new script

1. **Write the reference first** (`personal/<id>.txt`), with the exact
   punctuation you expect in the final result. Writing it before recording is
   what makes the punctuation ground truth (`punctuation: exact`).
2. Create `personal/<id>.roteiro.md` with the pause markers (3–5 s; mix
   pauses at sentence boundaries — where punctuation SHOULD survive — with
   pauses mid-clause — where punctuation should NOT appear).
3. Add the entry to `manifest.json` with `punctuation: "exact"` and the
   scenario (how many pauses, how long, and where).
4. This repo is private; if it's ever made public, decide beforehand whether
   the personal voice samples should be removed or replaced.

## Rules

- **Never change the audio or reference of an existing sample** — the value
  of the golden set is a comparable historical series. Made a mistake? Add
  another sample and retire the old one from the manifest (remove the entry,
  keep the file).
- Samples should be short (≤ 30 s) — the benchmark runs against a real
  provider and each request has a 10 s minimum billing on Groq.
