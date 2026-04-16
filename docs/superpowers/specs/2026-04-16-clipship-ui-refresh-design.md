# Clipship UI Refresh + Shortcut Recorder + Double-Tap

Date: 2026-04-16
Scope: rework the settings window to use a UI component framework, replace the
free-text shortcut field with a key-capture recorder, and add an optional
"double-tap to trigger" modifier for the global shortcut.

## Goals

1. The settings window looks like a modern desktop settings panel rather than a
   raw HTML form.
2. The global shortcut is set by pressing keys, not typing a string.
3. The shortcut can optionally require a "double-tap" pattern (press twice
   quickly) before firing, for users who want to avoid accidental triggers.

## Non-goals

- Dark mode toggle (leave room for it but don't implement)
- Localization / i18n
- Rewriting any upload / SSH / tray logic

## UI framework

Adopt **Tailwind CSS v3 + shadcn-svelte** (uses `bits-ui` under the hood).

- Tailwind via PostCSS, configured in `tailwind.config.js`, scanning
  `./src/**/*.{svelte,ts}` and `./index.html`.
- shadcn-svelte CLI (`pnpm dlx shadcn-svelte@latest init`) generates:
  - `src/lib/utils.ts` (cn helper)
  - `src/lib/components/ui/<component>/...` — components copied into the repo,
    no runtime dependency on the CLI.
- Only the components we use are added: `button`, `input`, `label`, `switch`,
  `card`. Add more later if needed.
- Color scheme: **neutral**, default radius, light mode only for v1.
- Existing `src/App.svelte` section components (`SshSection`, `DestinationSection`,
  `ShortcutSection`, `StatusArea`) rewritten to use Card + Label + Input +
  Button + Switch instead of `<fieldset>` / `<legend>` / raw inputs.
- Existing custom `<style>` blocks removed once replaced with Tailwind classes.

### File moves

- Delete the per-component ad-hoc styles in `App.svelte`.
- Keep the four section component files; change their internals to shadcn-svelte
  primitives.

## Shortcut recorder

Replaces the current free-text `<input bind:value={cfg.shortcut} />`.

### Component

New `src/components/ShortcutRecorder.svelte`. Props:

```ts
{ value: string, onChange: (v: string) => void }
```

Two visual states:

1. **Idle** — shows current accelerator as a chip + hint "Click to change".
2. **Recording** — yellow highlight, label "Press shortcut…", Esc cancels.

### Capture logic

- On click, enter `recording`.
- Listen to `window.addEventListener('keydown', …, { capture: true })`.
- Track currently-held modifiers via `event.ctrlKey / metaKey / altKey / shiftKey`.
- When the first **non-modifier** key fires:
  - If no modifier is held → reject, briefly flash error, keep recording.
  - Else build the accelerator string, call `onChange`, exit recording.
- `Escape` with no modifier → cancel, keep previous value, exit recording.
- `blur` on the recorder root → same as cancel.
- `event.preventDefault()` + `stopPropagation()` on every keydown while
  recording, so browser shortcuts don't interfere.

### Accelerator format

Keeps Tauri's accelerator string unchanged so `shortcut::register` need not
change:

- Modifier tokens: `CmdOrCtrl` (Ctrl on Windows/Linux, Cmd on macOS), `Alt`,
  `Shift`, `Super` (for Meta on non-macOS).
- Key tokens: letters `A`–`Z`, digits `0`–`9`, `F1`–`F24`, named keys
  `Space`, `Enter`, `Tab`, `Backspace`, `Delete`, arrow keys, etc.
- Mapping from `KeyboardEvent.code` to Tauri key tokens lives in a single
  `src/lib/shortcut/map.ts` file with a small lookup table + unit tests.

### Validation

Already present: backend `validate::shortcut`. Front end pre-validates the
captured combo so the user sees errors immediately; back end remains source of
truth.

## Double-tap option

### Config

Add to `Config`:

```rust
pub shortcut_double_tap: bool,  // default false
```

and the matching TS field. Because we gate loader on `version: 1`, and we want
to keep deserialization of existing v1 configs working, mark the field with
`#[serde(default)]`.

### UI

Below the shortcut recorder:

```
[Shortcut: Ctrl+Shift+U]  [ Change ]
[ Switch ] Require double-tap to trigger
          (hint: press the shortcut twice quickly)
```

### Semantics

When enabled, the registered shortcut fires upload only if two Pressed
transitions happen within **400 ms** of each other. The gap is specified: any
one key in the combination may be released and pressed again while the others
remain held. Example for `Ctrl+D`:

- Hold Ctrl, tap D twice within 400 ms ✓
- Hold D, tap Ctrl twice within 400 ms ✓
- Hold the whole combo continuously — only counts as the first press, does not
  retrigger ✗

### Backend implementation

In `src-tauri/src/shortcut.rs` (and `AppState`):

- Add `last_shortcut_press: Mutex<Option<Instant>>` to `AppState` (or a new
  `ShortcutState` struct owned by AppState). Value is `None` initially.
- In the shortcut handler, branch on `cfg.shortcut_double_tap`:
  - **false** — current behavior: on `Pressed` transition, run upload.
  - **true** — on `Pressed` transition:
    - Take the lock, read previous `Instant`.
    - If present and `now.duration_since(prev) <= 400 ms`, clear it and fire
      upload.
    - Otherwise, write `Some(now)` and do not fire.
- Config flag access: the handler reads `cfg.shortcut_double_tap` from a
  synchronously-loaded latest config (same `config::load` call already used by
  the upload path). This avoids having to re-register the shortcut when only
  the double-tap toggle changes.

### Key-repeat concern

OS key-repeat on held keys can generate repeated `Pressed` events for the
registered accelerator on some platforms. Tauri's
`tauri-plugin-global-shortcut` only fires on state transitions, so a held combo
should fire once. This spec assumes that behavior. If the assumption fails on
Windows in practice, the fallback (tracked as an implementation risk, not a
separate design path) is to require an intermediate `Released` between the two
`Pressed` events, using `state == Pressed` with a `last_release` timestamp.

## Data flow summary

```
[Recorder] keydown → accelerator string → cfg.shortcut
[Switch]   change   → cfg.shortcut_double_tap
[Save]     invoke("save_config", cfg)
           → Rust validates + persists + re-registers shortcut
[Shortcut] fires → handler reads cfg.shortcut_double_tap
           → if off: upload
           → if on:  compare against last_press, fire on 2nd within 400 ms
```

## Testing

- **Frontend unit**: `map.ts` accelerator building (modifier ordering,
  `KeyCode` → token). No DOM harness needed for pure function.
- **Frontend manual**: record in recorder; Esc; blur; rejecting no-modifier
  combos.
- **Backend unit**: new test in `shortcut.rs` (or a small extractable helper)
  for the 400 ms double-tap decision. Helper signature:

  ```rust
  pub fn should_fire(prev: Option<Instant>, now: Instant, window: Duration)
      -> (bool /* fire */, Option<Instant> /* new state */)
  ```

  which both the handler and the test call.
- **Backend integration**: existing `test_connection` etc. unaffected.
- **Config round-trip**: extend existing round-trip test to assert
  `shortcut_double_tap` defaults to `false` when missing from persisted JSON.

## Rollout

Single branch, no feature flag. Config migration is a no-op because the field
defaults.

## Out of scope / deferred

- Multiple shortcuts
- Per-profile shortcuts
- Recording non-ASCII or IME characters
- Sequence shortcuts (A then B)
- Configurable double-tap window
