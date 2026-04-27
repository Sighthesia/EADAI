# Variable Card Tuning Contract

> Executable contract for inline tuning on existing variable cards.

---

## 1. Scope / Trigger

- Trigger: the UI adds inline adjustment behavior to existing `VariablesPanel` cards.
- This contract applies only to cards that safely match a known tuning variable and expose the inline editor/step controls.
- Do not introduce separate tuning-only cards.

---

## 2. Signatures

- Eligible variable names: `pid_r`, `pid_p`, `pid_y`, `pid_t`, `pid_i`, `pid_d`.
- Eligibility gate: the variable must also have a finite numeric value.
- Command dispatch path: `sendBmi088Command('SET_TUNING', payload)`.
- Payload format: `key=value`, where `key` is the variable name and `value` is the formatted numeric text.

Example:

```ts
sendBmi088Command('SET_TUNING', `${name}=${formattedValue}`)
```

---

## 3. Contracts

- The current value text on an eligible card must be underlined to signal it is editable.
- Clicking the underlined current value opens the inline numeric editor/popover for that card.
- The metric row must keep value, trend arrow, and unit on a single line to preserve card height and prevent list jitter.
- Non-adjustable cards must render the current value as plain text, with no underline and no click affordance.
- The editor must include decrement, numeric input, and increment controls.
- The card footer must also expose decrement, step, and increment controls for the same adjustable variables.
- The footer step control uses the configured tuning step, not a free-form delta.
- The editor should focus/select the numeric input when opened so the current value can be replaced quickly.
- Non-eligible cards render as normal variable cards and do not expose tuning affordances.

---

## 4. Validation & Error Matrix

- Variable name not in the allowlist -> no tuning UI, no tuning command.
- Variable value is non-finite -> no tuning UI, no tuning command.
- Draft text is not finite on commit -> ignore the commit.
- Step action on a non-eligible card -> no-op.
- Embedded editor/button events must not bubble into the parent card selection or context-menu handlers.

---

## 5. Good/Base/Bad Cases

- Good: an eligible finite `pid_*` card shows an underlined current value, opens the numeric editor on click, and dispatches `SET_TUNING` with `pid_r=1.25`.
- Base: a non-adjustable variable card still behaves like a normal selectable card.
- Bad: creating a separate tuning card surface for the same variable.
- Bad: letting footer or input clicks toggle the whole card or open the context menu.

---

## 6. Tests Required

- Frontend build: `npm --prefix ui run build`
  - Verify the variables panel still compiles and the UI bridge types stay aligned.
- Manual interaction check:
  - Eligible values are underlined.
  - Clicking the value opens the editor.
  - Footer and editor step buttons update the draft and dispatch the tuning command.
  - Embedded controls do not toggle/select the card.
- Visual verification:
  - Adjustable and non-adjustable cards keep the same row height while values update.
  - The metric row does not wrap the value, arrow, or unit onto separate lines.
- Backend wiring check:
  - Confirm `SET_TUNING` still accepts plain text payload bytes and receives `key=value`.

---

## 7. Wrong vs Correct

### Wrong

```tsx
<article onClick={toggleCard}>
  <button onClick={openEditor}>value</button>
  <button onClick={step}>+</button>
</article>
```

### Correct

```tsx
<button onPointerDown={(event) => event.stopPropagation()} onClick={openEditor}>
  <strong className="adjustable">value</strong>
</button>
<button onPointerDown={(event) => event.stopPropagation()} onClick={step}>+</button>
```

---

## 8. Related Files

- `ui/src/components/VariablesPanel.tsx`
- `ui/src/styles.css`
- `src-tauri/src/model.rs`
