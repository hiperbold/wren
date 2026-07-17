---
title: "Wren Documentation"
description: "Index of Wren's project documentation: product, architecture, decisions, guides, design briefs, and reference material."
---

# Wren Documentation

Wren is a desktop dictation app where speech-to-text is a pluggable
**provider** — cloud API, local server, or embedded offline engine — chosen at
runtime, never hardcoded. This directory documents the product, the
architecture, and the reasoning behind it.

It's organized so each section answers a different kind of question. That
also makes it straightforward to publish as a static docs site (e.g.
[Docusaurus](https://docusaurus.io/)) later: each subfolder maps to a sidebar
category, and every file carries `title`/`description` frontmatter.

## Start here

- **[Roadmap](./roadmap.md)** — the phased build order, and what's shipped vs. planned.
- **[Product Overview](./product/overview.md)** — what Wren is, who it's for, and its scope.

## Architecture

How the system is built, and why a provider never gets coupled to the core.

- **[Architecture Overview](./architecture/overview.md)** — hexagonal (ports & adapters) design, the core, and the ports.
- **[Provider Model](./architecture/provider-model.md)** — the `Transcriber` contract that makes cloud/local/embedded interchangeable.
- **[Embedded Engine](./architecture/embedded-engine.md)** — how offline transcription works: the worker subprocess, IPC protocol, and model management.

## Decisions

- **[Decision Log](./decisions/README.md)** — Architecture Decision Records (ADRs): what's been decided, and why.

## Guides

- **[Transcription Quality Guide](./guides/transcription-quality.md)** — audio preprocessing, VAD, hallucination mitigation, and benchmarking.

## Design briefs

Forward-looking design documents for features not yet implemented.

- **[Context-Aware Post-Processing](./design/context-aware-post-processing.md)** — per-app/window formatting profiles (Phase 3).

## Reference

- **[Resource Budget](./reference/resource-budget.md)** — testable memory/latency budgets per app state.
- **[Open Questions & Known Technical Debt](./reference/open-questions.md)** — what's genuinely still unresolved.

## Research

- **[Case Study: Handy](./research/handy-case-study.md)** — the local-only dictation app that motivated Wren's pluggable-provider design.

## Conventions

- All documentation is in English (see [AGENTS.md](../AGENTS.md) for the project's language rules — PT-BR test fixtures are the only deliberate exception).
- Each file starts with YAML frontmatter (`title`, `description`).
- Decisions that have been made live in `decisions/` as numbered ADRs and are not rewritten after acceptance; open questions live in `reference/open-questions.md`.
- Design briefs in `design/` describe intent for future work, not current behavior.
