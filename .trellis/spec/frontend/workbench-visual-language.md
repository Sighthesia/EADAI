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

## Convention: Terminal Workspace Uses Vertical RX/TX Halves

**What**: Terminal-style runtime surfaces should keep a compact top summary and then split the main workspace into an upper receive half and a lower send half.

**Why**: RX content competes with TX controls if they share the same visual weight; splitting the workspace makes the send path feel like an action area and the receive path feel like an observation area.

**Example**:
```tsx
<section className="runtime-terminal-shell">
  <div className="runtime-terminal-summary-strip">...</div>
  <div className="runtime-terminal-workspace">
    <section className="runtime-terminal-half runtime-terminal-receive-half">...</section>
    <section className="runtime-terminal-half runtime-terminal-send-half">...</section>
  </div>
</section>
```

**Related**: `ui/src/components/RuntimeConsoleSection.tsx`, `ui/src/styles.css`

## Convention: Keep Repeated Metadata Single-Sourced

**What**: If protocol/device/telemetry state is already visible in one section, do not repeat it as another full card in the same panel.

**Why**: Repetition creates visual noise and weakens the hierarchy between primary and secondary information.

**Related**: `ui/src/components/RuntimePanel.tsx`, `ui/src/components/ScriptsPanel.tsx`, `ui/src/components/VariablesPanel.tsx`

## Convention: Command Lists Template Parameters In Place

**What**: Clicking a parameterless runtime command should execute immediately; clicking a parameterized command should prefill the composer with a useful template and focus the editable portion for quick finishing.

**Why**: Commands without arguments should feel like one-click actions, while commands with arguments should reduce typing and keep the user in flow.

**Example**:
```tsx
const { template, selection } = buildRuntimeCommandTemplateSelection(item)
onCommandInputChange(template)
input.setSelectionRange(selection[0], selection[1])
```

**Related**: `ui/src/components/RuntimeConsoleSection.tsx`, `ui/src/components/runtimeUtils.tsx`
