---
title: "ADR 0006: Single Unified App, No Separate Editions"
description: "Why Wren ships as one binary with the embedded engine always built in, rather than separate lightweight/offline editions."
---

# ADR 0006: Single Unified App, No Separate Editions

## Status

Accepted — decided 2026-07-12, superseding an earlier two-edition plan.

## Context

The original architecture considered shipping two separate build editions via Cargo feature `embedded`:

- **Lightweight edition**: cloud-only, using remote providers exclusively
- **Offline edition**: bundling the embedded ML engine and ONNX runtime

The motivation was to avoid shipping the ONNX runtime (~25 MB) to users who rely solely on cloud-based transcription providers.

## Decision

Wren ships as a single unified application with the embedded engine **always built in**. The engine functions as just another selectable provider at runtime, alongside remote and local-server providers.

- Provider selection is a **runtime configuration choice**, not a binary/installation choice.
- The Cargo feature `embedded` scheme was discarded.
- The Cargo feature `inference` survives **only within the `wren-embedded` crate**, for model-management tests (`models.rs`) to run without compiling ONNX; normal app builds always activate it.

This decision was made safe by the disposable-subprocess architecture ([ADR 0005](./0005-disposable-worker-subprocess.md)): the main process never loads ONNX when the user selects a cloud provider, because inference runs in an ephemeral subprocess that terminates when unused.

## Consequences

- **Simpler distribution and maintenance**: one binary, no feature-matrix decision tree at install time.
- **Zero runtime cost for cloud users**: the resident process never loads ONNX; the cost exists only in the binary footprint (~25 MB for ONNX runtime, deemed acceptable).
- **Frictionless provider switching**: users can change from embedded to cloud (or vice versa) through settings without reinstalling.
- **Implementation clarity**: the `wren-embedded` crate ([Embedded Engine](../architecture/embedded-engine.md)) is always present and built, removing conditional-compilation complexity from the core app.
