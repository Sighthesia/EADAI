# Directory Structure

> How backend code is organized in this project.

---

## Overview

The backend is currently a single Rust binary crate with one executable entry point at `src/main.rs`.
There are no established service, database, API, or utility submodules yet, so new backend code should start small and be split only when a clear boundary appears.

---

## Directory Layout

```
.
├── README.md
├── ROADMAP.md
├── src/
│   └── main.rs
└── .trellis/
    ├── spec/backend/
    └── scripts/
```

---

## Module Organization

There is no formal backend module layout yet.
Keep the entrypoint in `src/main.rs` until the backend grows enough to justify extraction into focused Rust modules.
Use one module per responsibility when splitting code, and prefer short, purpose-built files over broad utility dumping grounds.

---

## Naming Conventions

Use Rust standard naming: `snake_case` for files and modules, `CamelCase` for types, and descriptive function names.
Keep directory names lowercase and descriptive.
Documented project scaffolding uses kebab-case for Trellis task folders, such as `.trellis/tasks/00-bootstrap-guidelines/`.

---

## Examples

Current examples:

- `src/main.rs` for the binary entry point
- `.trellis/spec/backend/index.md` for backend documentation structure
- `.trellis/scripts/create_bootstrap.py` for project bootstrap automation

Anti-patterns:

- Adding unrelated logic directly into `src/main.rs` once the file starts growing
- Creating vague catch-all folders like `utils/` before a real use case exists
- Introducing nested module trees before there is a stable backend boundary
