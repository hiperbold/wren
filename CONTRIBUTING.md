# Contributing to Wren

Thanks for your interest in improving Wren! This guide covers how to get set up
and the conventions we follow. For a terse, machine-readable command reference
(handy for AI coding agents too), see [AGENTS.md](AGENTS.md).

## Getting started

**Prerequisites**

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Node.js](https://nodejs.org/) 20+ and npm
- Tauri v2 system dependencies — see the
  [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

**Set up**

```bash
git clone https://github.com/rafaelvieiras/wren.git
cd wren
cargo build            # build the Rust crates
cd apps/desktop
npm install
npm run app            # run the full app (tauri dev)
```

You can iterate on the UI alone, without the native backend, using the mock
mode: `npm run dev:mock` serves the UI at http://localhost:5173 with a fake
backend.

## Before you open a PR

Please make sure these pass:

```bash
# From the repository root
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test

# From apps/desktop
npm run build          # tsc --noEmit + vite build
```

## Conventions

- **Language.** All code, comments, and identifiers are in **English**. The only
  intentional exception is the golden test data under `tests/fixtures/`, which is
  PT-BR speech paired with audio — don't translate it.
- **UI strings are internationalized.** Don't hardcode display text; route it
  through `react-i18next` (`t("...")`) and add the key to **both** the `en` and
  `pt-BR` locale files under `apps/desktop/src/i18n/locales/`. English is the
  source language. See [AGENTS.md](AGENTS.md#language--i18n-rules) for the
  namespace layout.
- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/):
  `feat(ui): ...`, `fix(audio): ...`, `refactor(core): ...`, `docs: ...`,
  `chore: ...`.
- **Rust** should stay clean under `cargo clippy` and `cargo fmt`.
- **TypeScript/React**: prefer clear names and types over comments; comment the
  *why*, not the *what*. Match the style of the file you're editing.

## Reporting issues

- **Bugs and features:** open an issue using the templates.
- **Security vulnerabilities:** please do **not** open a public issue — see
  [SECURITY.md](SECURITY.md).

## Code of Conduct

By participating in this project you agree to abide by our
[Code of Conduct](CODE_OF_CONDUCT.md).
