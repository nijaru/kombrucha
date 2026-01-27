# Sprint Plan: Performance & Ruby Interop

Source: ai/TODO.md, docs/architecture/SPEC.md
Generated: 2026-01-26

## Overview

Continuing the roadmap with focus on performance optimizations, architectural modularization, and the foundation for Phase 5 (Source Builds) via Ruby interop.

---

## Sprint 4: Performance & Modularization (COMPLETED)

**Goal**: Optimize core commands and improve codebase structure.

### Tasks

- [x] TASK-15: Parallelize `outdated()` queries
- [x] TASK-16: Implement parallel extraction in `install`
- [x] TASK-17: Modularize `main.rs`
- [x] TASK-18: Implement batch dependency lookups

---

## Sprint 5: Ruby Interop Foundation (COMPLETED)

**Goal**: Embed the Ruby interpreter and load Homebrew formulae.

### Tasks

- [x] TASK-19: Setup Ruby VM with `magnus`
- [x] TASK-20: Implement Formula DSL loading
- [x] TASK-21: Map Formula metadata to Ruby objects

---

## Sprint 6: Source Build Execution (MVP) (COMPLETED)

**Goal**: Execute simple source build installation steps.

### Tasks

- [x] TASK-22: Implement Ruby-to-Rust bridge for DSL methods
- [x] TASK-23: Execute `install` method for a formula
- [x] TASK-24: Integrate source fallback in main installation loop

---

## Sprint 7: Formula DSL Completion

**Goal**: Support complex formula definitions and downloads.

### Tasks

- [ ] TASK-25: Implement `stable` block and version extraction
- [ ] TASK-26: Map `url` and `sha256` to Rust downloaders
- [ ] TASK-27: Implement resources and patches
- [ ] TASK-28: Add support for build-time options