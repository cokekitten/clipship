# Local Mode + Auto-Cleanup + UI Refinements — Design Spec

**Goal:** Add a local-save mode alongside SSH, an optional auto-cleanup background task, and several settings UI improvements.

**Architecture:** `Config` gains two new fields (`mode`, `auto_cleanup`). `UploadService` dispatches to SSH or local path based on mode. A background cleanup loop runs hourly and checks the flag on each tick. UI improvements are isolated to the frontend.

**Tech Stack:** Rust (Tauri 2), Svelte 5 runes, Tailwind CSS v3 + shadcn-svelte.

---

## 1. Config Changes

### New types — `src-tauri/src/config/mod.rs`

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    #[default]
    Ssh,
    Local,
}
```

Add to `Config`:

```rust
#[serde(default)]
pub mode: UploadMode,

#[serde(default)]
pub auto_cleanup: bool,
```

### Validation change

`Config::validate()` must only check SSH fields (host, port, username, private_key_path, remote_dir) when `self.mode == UploadMode::Ssh`. In local mode only `shortcut` is validated.

### Frontend types — `src/lib/types.ts`

```ts
export interface Config {
  version: 1;
  mode: "ssh" | "local";
  host: string;
  port: number;
  username: string;
  private_key_path: string;
  remote_dir: string;
  shortcut: string;
  shortcut_double_tap: boolean;
  auto_cleanup: boolean;
}
```

Default `cfg` state in `App.svelte` adds `mode: "ssh"` and `auto_cleanup: false`.

---

## 2. Local Upload Path

### `src-tauri/src/upload/service.rs`

`upload_inner()` dispatches on `cfg.mode` after the in-flight guard check:

```rust
match cfg.mode {
    UploadMode::Ssh => self.upload_ssh(cfg, content).await,
    UploadMode::Local => self.upload_local(content).await,
}
```

**`upload_ssh()`** — existing SSH logic extracted verbatim, no behaviour change.

**`upload_local()`**:
1. `classify(content)` — same as SSH path.
2. Destination directory: `std::env::temp_dir().join("clipship")`. Call `std::fs::create_dir_all()` on it.
3. Generate filename: `filename::build_remote_filename(&original_name)` (reuses timestamped naming).
4. Copy to destination:
   - `Classified::FileToUpload(p)` → `std::fs::copy(p, dest)`
   - `Classified::ImageBytes(bytes)` → `std::fs::write(dest, bytes)` (no temp_dir intermediary)
   - `Classified::DirectoryUnsupported` → `Err(UploadError::ClipboardDirectory)`
   - `Classified::Nothing` → `Err(UploadError::ClipboardEmpty)`
5. Write dest absolute path string to clipboard.
6. Update `last_uploaded`.
7. Return `UploadSuccess { remote_path: dest_str, clipboard_updated }` — the "remote path" is the local path; existing notification messages work unchanged.

**SSH binary check** (`ensure_ssh_scp`) is **skipped** in local mode — `run_shortcut_upload` and `trigger_upload_now` both call it before upload; they must check mode first and skip the binary check for local mode.

---

## 3. Auto-Cleanup

### New module — `src-tauri/src/cleanup/mod.rs`

```rust
pub fn is_ssh_complete(cfg: &Config) -> bool {
    !cfg.host.is_empty()
        && !cfg.username.is_empty()
        && !cfg.private_key_path.is_empty()
        && !cfg.remote_dir.is_empty()
}

/// Delete files in `dir` whose mtime is older than `max_age`. Silently skips
/// entries that cannot be stat'd or removed.
pub fn cleanup_local(dir: &Path, max_age: Duration) -> std::io::Result<()>

/// Run `ssh find <remote_dir> -maxdepth 1 -mtime +7 -type f -delete`.
/// No-op if `!is_ssh_complete(cfg)`.
pub async fn cleanup_remote(
    cfg: &Config,
    runner: &dyn CommandRunner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Errors are logged with `eprintln!` only — no user notification.

### Background loop — `src-tauri/src/lib.rs`

Spawn once in `setup()`:

```rust
let app_for_cleanup = handle.clone();
tauri::async_runtime::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.tick().await; // discard immediate first tick
    loop {
        interval.tick().await;
        let state = app_for_cleanup.state::<AppState>();
        let cfg = match crate::config::load(&state.config_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if !cfg.auto_cleanup {
            continue;
        }
        let local_dir = std::env::temp_dir().join("clipship");
        let _ = cleanup::cleanup_local(&local_dir, Duration::from_secs(7 * 24 * 3600));
        let runner = state.upload.runner.clone();
        let _ = cleanup::cleanup_remote(&cfg, runner.as_ref()).await;
    }
});
```

The loop re-reads config on every tick, so toggling `auto_cleanup` off in settings takes effect within one hour with no task cancellation needed.

---

## 4. Tray Menu

### Remove items — `src-tauri/src/tray.rs`

Remove the "Test connection" menu item (`test_conn`) and its event handler branch (`"test" => spawn_test(...)`). Remove the `spawn_test()` function entirely.

The tray menu becomes: Upload clipboard now / Open settings / separator / Copy last uploaded path / status / separator / Quit.

---

## 5. Settings UI

### `src/App.svelte` — header row

Replace:
```svelte
<h1 class="text-xl font-semibold">Clipship</h1>
```
With a flex row containing the title and Save button:
```svelte
<div class="flex items-center justify-between">
  <h1 class="text-xl font-semibold">Clipship</h1>
  <Button onclick={onSave}>Save</Button>
</div>
```
Remove the old button row at the bottom.

### Mode toggle — below the header

A segmented button pair (SSH / Local) rendered as two `<Button>` elements that toggle `cfg.mode`. Active button uses default variant, inactive uses `variant="outline"`:

```svelte
<div class="flex gap-1 rounded-md border p-1 w-fit">
  <Button
    variant={cfg.mode === "ssh" ? "default" : "ghost"}
    onclick={() => cfg.mode = "ssh"}
    size="sm">SSH</Button>
  <Button
    variant={cfg.mode === "local" ? "default" : "ghost"}
    onclick={() => cfg.mode = "local"
    size="sm">Local</Button>
</div>
```

### SSH section dimming

Wrap `<SshSection>` and `<DestinationSection>` in:
```svelte
<div class={cfg.mode === "local" ? "pointer-events-none opacity-50" : ""}>
  <SshSection bind:cfg />
  <DestinationSection bind:cfg />
</div>
```

### Test connection button

Only render when `cfg.mode === "ssh"`:
```svelte
{#if cfg.mode === "ssh"}
  <Button variant="secondary" onclick={onTest}>Test connection</Button>
{/if}
```
Move this button into the header area or keep as a standalone row below the dimmed SSH block — not in the bottom button row (which is removed).

### Switch rows — label alignment + click scope

All rows with a Switch currently use `justify-between` and clicking the Label triggers the switch. Fix:
- Label gets `class="flex flex-col gap-1 items-start"` (left-aligned title + description)
- Remove `for` attribute from `<Label>` — this breaks the label-click-activates-switch behaviour
- The `<Switch>` itself handles its own click; do not wrap the row in a clickable element

Apply this fix to: Launch at login, Double-tap, Auto-cleanup.

### Auto-cleanup toggle

Add to the System card in `App.svelte` below the Launch at login row:

```svelte
<Card.Content class="flex items-center justify-between">
  <div class="flex flex-col gap-1">
    <span class="text-sm font-medium">Auto-cleanup</span>
    <span class="text-xs text-muted-foreground">
      Delete files older than 7 days every hour. Remote cleanup requires SSH config to be complete.
    </span>
  </div>
  <Switch checked={cfg.auto_cleanup} onCheckedChange={(v) => cfg.auto_cleanup = v} />
</Card.Content>
```

### Hide scrollbar — `src/app.css` or `src/App.svelte`

Add to the `<main>` element or globally:
```css
/* app.css */
* {
  scrollbar-width: none; /* Firefox */
}
*::-webkit-scrollbar {
  display: none; /* Chrome/Safari/Edge */
}
```

---

## Out of Scope

- Configurable cleanup interval or retention period (fixed: 1 hour / 7 days)
- "Check for updates" tray item (not added)
- Local-mode specific tray label changes
