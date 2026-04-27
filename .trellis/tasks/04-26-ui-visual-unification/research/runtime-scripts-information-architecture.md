# Runtime and Scripts Information Architecture

## Problem

`Runtime` and `Scripts` both feel overloaded because they repeat the same device, protocol, command, and definition context while mixing different jobs in one scroll surface.

## Current Runtime Blocks

- Runtime header and active/parser chips
- Overview summary cards and flow strip
- Command center
- Raw serial inspector
- Protocol inspector
- Device catalog
- Runtime activity / triggers
- Definition link summary

## Current Scripts Blocks

- Scripts header and commands/fields chips
- Summary cards
- Lane switcher
- Definition browser
- Definition editor
- Runtime context preview
- Draft action toolbar

## Why It Feels Busy

- The same metadata appears in multiple places: protocol, commands, telemetry, parser, counts, latest update.
- Primary tasks and secondary reference data have equal visual weight.
- `Runtime` mixes observe, act, diagnose, and explain.
- `Scripts` mixes browse, edit, summarize, and runtime preview.
- Cross-panel duplication weakens the boundary between runtime operation and definition authoring.

## Recommended Direction

### Runtime

Restructure around:

1. Observe
2. Act
3. Diagnose

Implications:

- Keep one top summary block.
- Keep command sending as the primary workspace.
- Keep serial inspection nearby but secondary.
- Move protocol detail, catalog detail, trigger history, and definition-link explanation behind progressive disclosure.

### Scripts

Restructure around:

1. Browse
2. Edit
3. Runtime impact

Implications:

- Replace the large summary-card deck with one compact authoring overview.
- Keep lane switching and definition list together as the browse area.
- Keep the editor as the dominant pane.
- Demote runtime context into a compact impact panel instead of a competing summary surface.

## Design Rule

One top summary block plus one primary workspace block per panel. Repeated metadata should live in one place only, and lower-priority detail should be hidden behind `details` or a similar progressive-disclosure pattern.
