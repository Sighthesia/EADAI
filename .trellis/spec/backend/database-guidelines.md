# Database Guidelines

> Database patterns and conventions for this project.

---

## Overview

There is no database layer in the current codebase.
`src/main.rs` does not depend on an ORM, query builder, or migration tool, and no schema files exist yet.
When persistence is added, document the chosen library and migration workflow here before introducing application data models.

---

## Query Patterns

No query patterns are defined yet because the project does not currently query a database.
Avoid inventing a persistence abstraction until there is an actual storage use case.

---

## Migrations

No migration workflow exists yet.
If a database is introduced, record the exact command used to create, apply, and roll back migrations, plus where migration files live.

---

## Naming Conventions

No table or column naming conventions exist yet.
When persistence is added, keep names explicit and consistent with the selected Rust data model and schema tooling.

---

## Common Mistakes

Database-related anti-patterns to avoid:

- Adding persistence dependencies before the data model is stable
- Hard-coding schema assumptions in `main.rs`
- Mixing storage code into unrelated runtime setup

(To be filled by the team)
