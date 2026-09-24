# Fileport Icon + Auto-Paste + Tray Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Fileport icon set (bundle + tray template), add the opt-in `auto_paste` shortcut behavior, and fix tray presentation (tooltip, macOS theme-adaptive mono glyph).

**Architecture:** Icon assets are generated from the two checked-in SVGs (`public/app-icon.svg`, `public/tray-glyph.svg`) via `pnpm tauri icon` and a headless-Chrome PNG render. Auto-paste is a config-gated post-step of `shortcut::run_shortcut_upload` only, expressed as a pure `should_paste` decision plus an enigo keystroke helper in a new `paste` module.

**Tech Stack:** Tauri 2 (tray, global-shortcut), Rust (enigo for keystrokes), Svelte 5 + Tailwind (shadcn-svelte switch/label), `pnpm tauri icon` CLI.

**Spec:** `docs/superpowers/specs/2026-09-25-clipship-icon-auto-paste-design.md`

## Global Constraints

- Auto-paste fires **only** on the global-shortcut path and **only** when `UploadSuccess.clipboard_updated == true`. Tray menu / settings UI never auto-paste.
- `auto_paste` defaults to `false`; config version stays 1 (absent field = off).
- macOS tray icon is a template image (`icon_as_template(true)`); Windows/Linux keep `app.default_window_icon()`.
- Tray tooltip text is `Clipship` on all platforms.
- Paste failure must never affect the upload; it adds one `Message::PasteFailed` notification only.
- Keystroke: Cmd+V on macOS, Ctrl+V elsewhere; release modifiers (Control/Alt/Shift/Meta) first; ~100 ms delay before sending keys.
- All icons are pure SVG (no external fonts/images/filters); generated files live in `src-tauri/icons/` with the existing filenames; `tauri.conf.json` icon list is unchanged.

---

### Task 1: Icon assets + tray + favicon + in-app logo

**Files:**
- Generate: `src-tauri/icons/*` (from `public/app-icon.svg`), `src-tauri/icons/tray-template.png` (from `public/tray-glyph.svg`)
- Modify: `src-tauri/src/tray.rs` (tray builder: icon selection + template flag + tooltip)
- Modify: `index.html` (favicon link)
- Modify: `src/App.svelte` (logo `<img>` in header)

**Interfaces:**
- Consumes: `public/app-icon.svg` (1024 badge), `public/tray-glyph.svg` (24×24 `#000` glyph)
- Produces: `src-tauri/icons/tray-template.png` — 44×44 black-on-transparent PNG, embedded via `include_bytes!` in Task 1.

- [ ] **Step 1: Generate bundle icons**

```bash
cd /Users/cokekitten/dev/clipship
npx --no-install tauri icon public/app-icon.svg
rm -rf src-tauri/icons/android src-tauri/icons/ios
file src-tauri/icons/icon.icns src-tauri/icons/icon.ico
```
Expected: icons regenerated (old Tauri default replaced); `icon.icns` / `icon.ico` reported as valid.

- [ ] **Step 2: Generate `tray-template.png`**

```bash
cat > /tmp/clipship-tray-render.html <<'EOF'
<!doctype html><html><head><style>html,body{margin:0;padding:0;background:transparent}img{display:block;width:44px;height:44px}</style></head>
<body><img src="file:///Users/cokekitten/dev/clipship/public/tray-glyph.svg"></body></html>
EOF
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless --disable-gpu \
  --screenshot=/Users/cokekitten/dev/clipship/src-tauri/icons/tray-template.png \
  --window-size=44,44 --hide-scrollbars --default-background-color=00000000 \
  "file:///tmp/clipship-tray-render.html"
file src-tauri/icons/tray-template.png
```
Expected: `PNG image data, 44 x 44, 8-bit/color RGBA` (alpha background, black glyph).

- [ ] **Step 3: Update the tray builder**

In `src-tauri/src/tray.rs`, replace the icon line of `TrayIconBuilder` and add template flag + tooltip. Replace:

```rust
    let _tray = TrayIconBuilder::with_id("clipship-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
```

with:

```rust
    #[cfg(target_os = "macos")]
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"));
    #[cfg(not(target_os = "macos"))]
    let icon = app.default_window_icon().unwrap().clone();

    let _tray = TrayIconBuilder::with_id("clipship-tray")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Clipship")
        .menu(&menu)
```

If the compiler reports `icon_as_template` or `tooltip` missing on the builder, use the `TrayIcon` setters (`set_icon_as_template`, `set_tooltip`) on the built tray instead — same semantics.

- [ ] **Step 4: Favicon**

In `index.html` `<head>`, add:

```html
    <link rel="icon" type="image/svg+xml" href="/app-icon.svg" />
```

- [ ] **Step 5: In-app logo**

In `src/App.svelte`, in the header div (`class="flex items-center gap-2"`), before the `<h1>`:

```svelte
      <img src="/app-icon.svg" alt="" class="size-6 rounded-md" />
```

- [ ] **Step 6: Compile check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: no errors. (If `Image::from_bytes` is rejected for missing PNG decode support, add `features = ["image-png"]` to the `tauri` dependency in `Cargo.toml` and re-run.)

- [ ] **Step 7: Commit**

```bash
git add -A src-tauri/icons src-tauri/src/tray.rs index.html src/App.svelte src-tauri/Cargo.toml
git commit -m "feat(icon): Fileport icon set — bundle icons, macOS template tray glyph, tooltip, favicon"
```

### Task 2: `auto_paste` config + settings switch (TDD)

**Files:**
- Modify: `src-tauri/src/config/mod.rs` (field, Default, tests)
- Modify: `src-tauri/src/commands.rs` (`save_general_config` copies `auto_paste`)
- Modify: `src/lib/types.ts`, `src/App.svelte`, `src/components/ShortcutSection.svelte`

**Interfaces:**
- Produces: `Config.auto_paste: bool` (serde default false), `ShortcutSection` Switch bound to `cfg.auto_paste`.

- [ ] **Step 1: Write failing tests** in `config/mod.rs` tests module (next to the `auto_cleanup` tests):

```rust
    #[test]
    fn auto_paste_defaults_to_false_when_field_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        ).unwrap();
        let cfg = load(&path).unwrap();
        assert!(!cfg.auto_paste);
    }

    #[test]
    fn round_trip_preserves_auto_paste() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.auto_paste = true;
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert!(back.auto_paste);
    }
```

- [ ] **Step 2: Run tests, expect compile failure** (`auto_paste` field unknown):

```bash
cargo test auto_paste 2>&1 | tail -5
```

- [ ] **Step 3: Implement.** In `config/mod.rs` add to `Config` (after `auto_cleanup`):

```rust
    #[serde(default)]
    pub auto_paste: bool,
```

and to `Default for Config` (after `auto_cleanup: false,`):

```rust
            auto_paste: false,
```

In `commands.rs` `save_general_config`, after `existing.auto_cleanup = cfg.auto_cleanup;`:

```rust
    existing.auto_paste = cfg.auto_paste;
```

In `src/lib/types.ts` `Config`, after `auto_cleanup: boolean;`:

```ts
  auto_paste: boolean;
```

In `src/App.svelte` initial `cfg` state, after `auto_cleanup: false,`:

```ts
    auto_paste: false,
```

In `src/components/ShortcutSection.svelte`, add handler and row (after the double-tap row):

```svelte
    <div class="flex items-center justify-between">
      <Label class="flex flex-col gap-1 items-start">
        <span>Paste path after upload</span>
        <span class="text-xs font-normal text-muted-foreground">
          After the shortcut uploads, paste the path into the frontmost app automatically.
          Only applies to the global shortcut. macOS requires the Accessibility permission.
        </span>
      </Label>
      <Switch
        checked={cfg.auto_paste}
        onCheckedChange={onAutoPasteChange}
      />
    </div>
```

```ts
  function onAutoPasteChange(v: boolean) {
    cfg.auto_paste = v;
  }
```

- [ ] **Step 4: Run tests, expect green**

```bash
cargo test auto_paste && cd .. && pnpm check 2>&1 | tail -3
```
Expected: 2 Rust tests pass; svelte-check clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/mod.rs src-tauri/src/commands.rs src/lib/types.ts src/App.svelte src/components/ShortcutSection.svelte
git commit -m "feat(config): auto_paste setting with Shortcut-section switch"
```

### Task 3: `paste` module + shortcut wiring + `PasteFailed` notification (TDD)

**Files:**
- Create: `src-tauri/src/paste/mod.rs`
- Modify: `src-tauri/Cargo.toml` (enigo), `src-tauri/src/lib.rs` (`pub mod paste;`)
- Modify: `src-tauri/src/shortcut/mod.rs` (`run_shortcut_upload`)
- Modify: `src-tauri/src/notify.rs` (`Message::PasteFailed`)

**Interfaces:**
- Consumes: `Config.auto_paste`, `UploadSuccess { remote_path, clipboard_updated }`
- Produces: `paste::should_paste(cfg: &Config, result: &Result<UploadSuccess, UploadError>) -> bool`;
  `paste::paste_after_delay() -> Result<(), String>` (100 ms sleep then `send_paste`);
  `Message::PasteFailed` (unit variant).

- [ ] **Step 1: Add dependency**

```bash
cd src-tauri && cargo add enigo
```
(If crates.io is unreachable, configure the rsproxy mirror for this run only: `CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse cargo add enigo --registry crates-io` or add `[source.crates-io] replace-with = "rsproxy"` to `~/.cargo/config.toml` — reversible config, remove after resolving.)

- [ ] **Step 2: Write failing tests** — create `src-tauri/src/paste/mod.rs` with the tests and stub, and add `pub mod paste;` to `src-tauri/src/lib.rs`:

```rust
use crate::config::Config;
use crate::upload::errors::UploadError;
use crate::upload::service::UploadSuccess;

/// Paste only when the user opted in AND the path actually reached the clipboard.
pub fn should_paste(cfg: &Config, result: &Result<UploadSuccess, UploadError>) -> bool {
    todo!()
}

/// Sleep briefly (the hotkey may still be held), then send the paste keystroke.
pub async fn paste_after_delay() -> Result<(), String> {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    send_paste()
}

fn send_paste() -> Result<(), String> {
    use enigo::{Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    // Release modifiers the hotkey may still hold so the paste is not seen as e.g. Cmd+Shift+V.
    for key in [Key::Shift, Key::Control, Key::Alt, Key::Meta] {
        let _ = enigo.key_up(key);
    }
    #[cfg(target_os = "macos")]
    let primary = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let primary = Key::Control;
    enigo.key_down(primary).map_err(|e| e.to_string())?;
    enigo.key_click(Key::Unicode('v')).map_err(|e| e.to_string())?;
    enigo.key_up(primary).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UploadMode;

    fn cfg(auto_paste: bool) -> Config {
        Config {
            version: 1,
            mode: UploadMode::Local,
            host: String::new(),
            port: 22,
            username: String::new(),
            private_key_path: String::new(),
            remote_dir: String::new(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
            shortcut_double_tap: false,
            auto_cleanup: false,
            auto_paste,
        }
    }

    fn success(clipboard_updated: bool) -> UploadSuccess {
        UploadSuccess { remote_path: "/tmp/x.png".into(), clipboard_updated }
    }

    #[test]
    fn off_never_pastes_even_on_success() {
        assert!(!should_paste(&cfg(false), &Ok(success(true))));
    }

    #[test]
    fn on_and_clipboard_updated_pastes() {
        assert!(should_paste(&cfg(true), &Ok(success(true))));
    }

    #[test]
    fn on_but_clipboard_changed_does_not_paste() {
        assert!(!should_paste(&cfg(true), &Ok(success(false))));
    }

    #[test]
    fn on_but_upload_failed_does_not_paste() {
        assert!(!should_paste(&cfg(true), &Err(UploadError::ClipboardEmpty)));
    }
}
```

Note: if `UploadService`'s struct-literal style in existing tests is needed for `success`, no — `UploadSuccess` is a plain pub struct with pub fields; the above is all that is needed. If the enigo API surface differs (0.2 vs 0.3), adapt `send_paste` only — `key_down`/`key_click`/`key_up` with `Key::Meta`/`Key::Control`/`Key::Unicode('v')` exist in both.

- [ ] **Step 3: Run tests, expect red**

```bash
cargo test paste 2>&1 | tail -5
```
Expected: `not yet implemented` panic in the 4 tests (or compile error in `send_paste` if the enigo API differs — fix `send_paste` first, keep `todo!()`).

- [ ] **Step 4: Minimal implementation** — replace `todo!()` with:

```rust
    cfg.auto_paste && matches!(result, Ok(s) if s.clipboard_updated)
```

- [ ] **Step 5: Run tests, expect green**

```bash
cargo test paste 2>&1 | tail -5
```

- [ ] **Step 6: Wire the shortcut path.** In `src-tauri/src/shortcut/mod.rs`, replace the tail of `run_shortcut_upload`:

```rust
    tray::set_status(&app, "Uploading\u{2026}");
    let result = state.upload.upload(&cfg).await;
    tray::set_status(&app, "Idle");
    if state.upload.last_uploaded.lock().unwrap().is_some() {
        tray::set_last_uploaded_enabled(&app, true);
    }
```

with:

```rust
    tray::set_status(&app, "Uploading\u{2026}");
    let result = state.upload.upload(&cfg).await;
    tray::set_status(&app, "Idle");
    if state.upload.last_uploaded.lock().unwrap().is_some() {
        tray::set_last_uploaded_enabled(&app, true);
    }
    if crate::paste::should_paste(&cfg, &result) {
        if crate::paste::paste_after_delay().await.is_err() {
            state.upload.notifier.notify(Message::PasteFailed);
        }
    }
```

(`Message` is already imported in that module.)

- [ ] **Step 7: Add the notification.** In `src-tauri/src/notify.rs`, add `PasteFailed,` to the enum (after `ClipboardWriteFailed`), and in `render` add:

```rust
        Message::PasteFailed => {
            #[cfg(target_os = "macos")]
            let hint = "Grant Clipship the Accessibility permission (System Settings \u{2192} Privacy & Security \u{2192} Accessibility).";
            #[cfg(not(target_os = "macos"))]
            let hint = "Your desktop session may block synthetic input.";
            (
                "Clipship",
                format!("Path copied, but auto-paste failed \u{2014} paste it manually. {hint}"),
            )
        }
```

- [ ] **Step 8: Full Rust suite green**

```bash
cargo test 2>&1 | tail -8
```
Expected: all tests pass (config 2 new + paste 4 new + existing).

- [ ] **Step 9: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/paste src-tauri/src/lib.rs src-tauri/src/shortcut/mod.rs src-tauri/src/notify.rs
git commit -m "feat(paste): opt-in auto-paste after shortcut upload (enigo, gated on clipboard_updated)"
```

### Task 4: README + full verification

**Files:**
- Modify: `README.md`
- Verify: frontend tests/check, release build

- [ ] **Step 1: README** — in Features add:

```markdown
- **Auto-paste (optional)** — after the shortcut uploads, paste the path into the frontmost app automatically. macOS requires the Accessibility permission (System Settings → Privacy & Security → Accessibility).
```

and in Usage, after step 6, add:

```markdown
> **Tip:** enable *Paste path after upload* in settings to have the path pasted straight into your CLI. On macOS, grant Clipship the Accessibility permission the first time.
```

- [ ] **Step 2: Frontend suites**

```bash
pnpm test 2>&1 | tail -4 && pnpm check 2>&1 | tail -3 && pnpm build 2>&1 | tail -3
```
Expected: vitest green, svelte-check clean, vite build ok.

- [ ] **Step 3: Release build (icon/bundle verification)**

```bash
pnpm tauri build 2>&1 | tail -6
```
Expected: bundler completes for dmg/msi/nsis config (msi/nsis may be skipped on macOS); icons embedded without error.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: auto-paste feature + macOS Accessibility note"
```
