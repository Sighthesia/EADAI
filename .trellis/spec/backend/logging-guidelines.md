# Logging Guidelines

> How logging is done in this project.

---

## Overview

The current codebase does not yet use a logging crate.
`src/main.rs` only prints the default Rust greeting, so there is no structured logging format, level policy, or redaction rule in place yet.
When logging is introduced, keep it structured and consistent across runtime components.

---

## Log Levels

No project-specific log levels are defined yet.
Use the standard meaning of `debug`, `info`, `warn`, and `error` once a logger is added.

---

## Structured Logging

No structured log format is defined yet.
When the backend grows, prefer key/value fields that make device state, channel identifiers, and request context searchable.

---

## What to Log

Log only the events that help explain runtime behavior, such as startup, device connection changes, parsing failures, and command execution outcomes.

---

## What NOT to Log

Do not log secrets, credentials, raw private data, or any payload that is not needed for debugging.
Avoid dumping large binary blobs or unbounded serial traffic into logs.
