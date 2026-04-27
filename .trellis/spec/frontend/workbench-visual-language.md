# Workbench Visual Language

## Scope

This guide covers the shared presentation pattern used by the workbench shell and docked panels.

## Convention: Compact, Task-First Chrome

**What**: Keep docked workspace surfaces focused on the current task, not on permanent explanatory copy.

**Why**: Constant helper text, repeated metadata, and stacked summary cards make the first viewport feel busy and unfocused.

**Example**:
```tsx
// Good: one compact status strip, then the primary workspace, then disclosure for detail.
<section className="panel">
  <header className="runtime-header">...</header>
  <main className="runtime-primary-layout">...</main>
  <details className="runtime-diagnostic-shell">...</details>
</section>
```

## Convention: Progressive Disclosure for Secondary Detail

**What**: Put protocol details, catalogs, trigger history, and other reference material behind `details` or equivalent collapsible sections.

**Why**: Secondary data stays available without competing with the primary action path.

**Example**:
```tsx
<details className="runtime-disclosure">
  <summary>
    <strong>Schema reference</strong>
    <small>12 fields · 100 Hz</small>
  </summary>
  <div className="runtime-disclosure-body">...</div>
</details>
```

## Convention: Summary Strip Over Summary Deck

**What**: Prefer one compact summary strip or a small number of status chips instead of several equal-weight summary cards.

**Why**: A deck of similar cards encourages scanning without prioritization; a strip gives the panel a clear top line.

**Example**:
```tsx
<div className="scripts-overview-header">
  <div>...</div>
  <div className="scripts-overview-metrics">...</div>
</div>
```

## Convention: Keep Repeated Metadata Single-Sourced

**What**: If protocol/device/telemetry state is already visible in one section, do not repeat it as another full card in the same panel.

**Why**: Repetition creates visual noise and weakens the hierarchy between primary and secondary information.

**Related**: `ui/src/components/RuntimePanel.tsx`, `ui/src/components/ScriptsPanel.tsx`, `ui/src/components/VariablesPanel.tsx`
