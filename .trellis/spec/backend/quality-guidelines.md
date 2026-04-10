# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

The backend is still at the scaffold stage, so quality guidance should stay simple and practical.
Prefer small Rust modules, explicit control flow, and code that can be understood without cross-file guesswork.

---

## Forbidden Patterns

Avoid:

- Large monolithic `main.rs` growth once behavior becomes non-trivial
- `unwrap()` in runtime paths that can fail in production
- Adding abstractions before there are at least two concrete use cases
- Copying roadmap ideas into code before they are needed

---

## Required Patterns

Keep backend code:

- Small enough to scan quickly
- Written in idiomatic Rust
- Separated by responsibility when a file starts to cover more than one concern
- Backed by comments only when the reason for a choice is not obvious

---

## Testing Requirements

There are no backend tests yet.
When backend logic is added, add tests for parsing, protocol handling, and any stateful behavior that can regress silently.

---

## Code Review Checklist

Reviewers should check that changes:

- Match the current minimal architecture instead of inventing layers
- Handle errors explicitly
- Avoid leaking secrets or oversized payloads into logs
- Include tests for non-trivial behavior
