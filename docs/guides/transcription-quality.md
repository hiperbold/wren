---
title: "Transcription Quality Guide"
description: "Practical techniques for cost, quality, and robustness in API-based transcription — audio preprocessing, VAD, hallucination mitigation, and benchmarking."
---

# Transcription Quality Guide

Sources: Groq official documentation, Groq cookbook, Whisper project discussions, and literature on hallucination (links at the end).

## 1. Groq Models and When to Use Each

| Model | Price | WER | Speed | Use Case |
|---|---|---|---|---|
| `whisper-large-v3` | US$0.111/h | ~10.3% | 189x real-time | Maximum precision (error-sensitive content) |
| `whisper-large-v3-turbo` | **US$0.04/h** | ~12% | 216x real-time | **Wren default** — best cost-quality ratio for multilingual |

Turbo costs ~89% less than OpenAI Whisper (US$0.36/h) with comparable quality. For everyday dictation, the WER difference rarely justifies 2.8x the price — LLM post-processing corrects more error per dollar.

### The True Cost of Dictation

- **Minimum billing: 10 seconds per request.** A 2-second dictation costs the same as 10 seconds.
- Average 10-second dictation ≈ US$0.00011. **100 dictations/day ≈ US$0.33/month.**
- Conclusion: per-use cost is negligible; waste comes from **useless requests** (silence, accidental triggers).

## 2. Audio Preprocessing (Direct Savings)

Groq **converts everything to 16 kHz mono** before transcribing. Sending more only wastes upload bandwidth and file size ceiling:

| Format | 1-minute Size | Note |
|---|---|---|
| WAV 48 kHz stereo 16-bit (raw capture format) | ~11.5 MB | free tier ceiling (25 MB) exceeded in ~2 min |
| WAV 16 kHz mono 16-bit | ~1.9 MB | 6x smaller, identical quality post-downsample |
| **FLAC 16 kHz mono** (recommended) | ~0.9–1.2 MB | lossless, ~10x smaller than raw |

Best practices:

- **Downmix to mono + resample to 16 kHz on the client** (capture via cpal may be 44.1/48 kHz stereo — convert before upload).
- **FLAC for lossless compression**; accepted formats: flac, mp3, mp4, mpeg, mpga, m4a, ogg, wav, webm. Never re-compress with loss (mp3/ogg) audio already in memory — FLAC gives the gain without quality cost.
- File ceiling: **25 MB (free) / 100 MB (dev)**. With FLAC 16k mono, free tier accommodates ~20+ minutes — irrelevant for dictation, important for future file transcription.
- **Long audio** (>10 min): split into ~10-minute chunks with **10-second overlap** (avoids cutting words at boundaries). Not applicable to typical dictation, but documented for future file transcription.

## 3. Don't Pay (or Hallucinate) for Silence — VAD

Whisper was trained on audio/text pairs where silent sections had arbitrary captions — so it **invents text for silence and noise** ("hallucination"). In PT-BR the classic is *"Legendas pela comunidade Amara.org"*; in Wren's first real test, ambient noise became *"E aí"*.

Standard market mitigations (by cost-benefit):

1. **VAD (Voice Activity Detection) before sending** — Silero VAD is the de facto standard (used by Handy). **Wren's decision:** earshot 1.x (pure-Rust neural VAD, ~110 KiB) in the base app — Silero would require ONNX Runtime ~11–25 MB per platform + 2.3 MB model, a cost that in Handy "was already paid" by local Whisper; Silero via `ort` is a plan-B opt-in if earshot fails in practice. Three roles:
   - **gate**: if the recording contains no speech, don't send it (saves minimum 10s and avoids hallucinating);
   - **trim**: removing silence from edges reduces billed duration and removes fertile hallucination ground (silence at end of audio). Padding is **asymmetric**: ~500 ms lead-in at start (Whisper decodes better with a breath before speech — at 200 ms the golden set regressed, WER 0→7% on the accent sample) and ~200 ms at end (leftover silence at the end is exactly where Whisper hallucinates);
   - **internal pause compression**: thinking pauses within dictation (5–20 s) longer than a configurable threshold (default 2 s; `compress_pauses_over_ms` in settings, `null` = off) are **shortened to a fixed residue of ~400 ms** — compress, not remove: the residue preserves the acoustic phrase-boundary cue the model uses to punctuate correctly. The ~200 ms padding around each speech segment remains intact (compression acts only in the gap interior). Gains: less billed duration from provider, less hallucination on internal pause, and less idle inference on local models. History records both durations ("42s dictated → 28s sent") to not misrepresent what was sent.
2. **Discard very short recordings** (<~300 ms of speech) — almost always accidental shortcut press.
3. **`temperature: 0`** — deterministic, less creative on ambiguous segment.
4. **Fix `language`** (ISO 639-1) — avoids wrong language detection on short speech, improves accuracy and latency.
5. VAD is not perfect (Silero mislabels part of pure noise) — LLM post-processing is the second line to filter residue.

## 4. API Parameters That Affect Quality

| Parameter | Recommendation | Status in Wren |
|---|---|---|
| `language` | fix when known | ✅ implemented (settings) |
| `temperature` | **0** | ✅ implemented in adapter |
| `prompt` | ≤224 tokens: proprietary vocabulary, names, acronyms, punctuation style | ⏳ planned (customizable glossary) |
| `response_format` | `json` (dictation); `verbose_json` for timestamps | ✅ `json` |
| `timestamp_granularities` | `word`/`segment` with `verbose_json` | ⏳ when needed (captions) |

On `prompt`: it is **not an instruction** ("transcribe well") — the model treats it as *prior context of the audio*. Correct use is seeding vocabulary and style: `"Wren, Tauri, wgpu, Groq, API key…"` makes the model spell technical terms correctly. It's the standard mechanism for per-app/profile glossaries.

## 5. Robustness and Operations (Market Standards)

- **Retry with exponential backoff** for 429/5xx (free tier: ~2,000 audio requests/day — intensive dictation doesn't approach it, but retry bursts do).
- **Generous but finite timeout** (Wren uses 60 s) and **clean cancellation**.
- **Declarative fallback between providers** (local → cloud or vice-versa) — already in roadmap; it's Wren's structural differentiator.
- **Never lose user audio**: recording goes to disk **before** sending; if transcription fails (network/provider), it's preserved in `recordings/` with a failure entry in history and **resubmission via UI**; on success it's deleted — failure is the only state that leaves audio on disk. ✅ implemented (`RecordingStore` port + `DictationService::retry`).
- **Never log audio or API keys**; log only metadata (duration, latency, status) — this is what `history.jsonl` already does.
- **Privacy**: explicit egress per provider. Check and document the data retention policy of the active provider — for the user, "where does my audio go and how long does it stay" is part of the provider choice.

## 6. Implementation Checklist for Wren

**Highest-impact-per-effort (shipped):**

- [x] Resample to **16 kHz mono** + encode **FLAC** before upload (`AudioClip` already isolates this — change only the capture adapter).
- [x] `temperature: 0` in `RemoteApiTranscriber`.
- [x] **VAD earshot** in `Vad` port: gate + trim.
- [x] Discard sessions with <300 ms of detected speech.
- [x] Internal pause compression: >configurable threshold (default 2 s) becomes ~400 ms residue; history shows dictated × sent duration. ✅
- [x] Retry with backoff (1 extra attempt) for 429/5xx.

**Planned (future refinements):**

- [ ] `prompt` with per-profile customizable glossary.
- [ ] Declarative fallback between providers.
- [ ] `verbose_json` + timestamps when use case exists.
- [ ] Chunking with overlap for long file transcription.

## 7. Benchmark and Golden Set

Any pipeline change (pause compression, model swap, post-processing) requires measurable before/after. The `wren-bench` runs the complete pipeline (normalize → VAD → FLAC → provider) over a fixed golden set and measures **WER** (content) and **PER** (punctuation) separately on purpose: LLM post-processing should lower PER **without raising WER** — a single metric would hide exactly that tradeoff.

- **Golden set**: `tests/fixtures/golden/` — manifest + audio + references; the protocol for adding samples is in its README. Public samples come from the tech4humans dataset (Apache-2.0), with curated punctuation.
- **How to run**: `scripts/bench-stt.sh` (active provider from settings.json; overrides via `WREN_BENCH_BASE_URL`/`WREN_BENCH_MODEL`/`WREN_BENCH_API_KEY`). Runs against real provider — uses quota.
- **Output**: one line per sample × VAD variant (no-vad, vad, vad+pauses) with WER%, PER%, capitalization divergences, recorded→sent duration (compression effect), and API latency.

## Sources

- [Groq — Speech to Text (official documentation)](https://console.groq.com/docs/speech-to-text)
- [Groq — pricing on-demand](https://groq.com/pricing)
- [Groq — Whisper Large v3 Turbo (announcement)](https://groq.com/blog/whisper-large-v3-turbo-now-available-on-groq-combining-speed-quality-for-speech-recognition)
- [Groq cookbook — Speech and Audio Processing](https://deepwiki.com/groq/groq-api-cookbook/2-speech-and-audio-processing)
- [Groq community — chunking longer audio files](https://community.groq.com/t/chunking-longer-audio-files-for-whisper-models-on-groq/162)
- [Whisper #679 — hallucination solutions](https://github.com/openai/whisper/discussions/679)
- [whisper.cpp #1724 — silence hallucination](https://github.com/ggml-org/whisper.cpp/issues/1724)
- [Calm-Whisper (paper) — hallucination in non-speech](https://arxiv.org/html/2505.12969v1)
- [Investigation of Whisper ASR Hallucinations (paper)](https://arxiv.org/pdf/2501.11378)
