# Project Start

Date: 2026-06-27

## Goal

Establish Ferrite's documentation and iteration model before writing inference
engine code.

## Context

The repository currently contains a minimal root `README.md`, a `LICENSE`, and
a baseline research corpus under `research/`. The research is a foundation, not
an implementation contract. Ferrite needs a project operating model so future
implementation, validation, benchmarking, ADRs, research, and theory work stay
traceable.

## Changes

- Added `documentation/README.md`.
- Added `documentation/engineering/ferrite-operating-model.md`.
- Added the initial project goal, later consolidated into
  `documentation/engineering/ferrite-operating-model.md`.
- Added ADR, development note, research note, theory note, and benchmark note
  indexes and templates.
- Added ADR 0001 to accept the documentation and iteration model.

## Validation

Documentation-only change. Validation should confirm:

- The documentation tree exists.
- Required process files are present.
- No contradictory draft language remains.

## Results

Ferrite now has a documented operating model for progressive CPU inference work.
It also has a reusable goal prompt for starting or resuming long-running Ferrite
sessions.

## Follow-Ups

- Write the first implementation milestone spec.
- Decide the initial Rust workspace shape.
- Decide the first Tier 0 model fixture and reference comparison method.
