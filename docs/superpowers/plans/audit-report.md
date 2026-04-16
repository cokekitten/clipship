# Plan Audit Report: Clipship v1 Implementation Plan

> **Scope:** This audit reviews the implementation plan `docs/superpowers/plans/2026-04-16-clipship-v1.md` against the design spec `docs/superpowers/specs/2026-04-16-clipship-design.md`. No implementation code exists in the repo yet, so findings are plan-quality / spec-drift issues that will surface during execution.

---

## Findings (ordered by severity)

### Critical

#### 1. Error-path notifications are completely missing
- **WI ref:** Task 13, Task 17, Task 19, Task 20
- **Evidence:**
  - `UploadService::upload` (Task 13) calls `self.notifier.notify(...)` only on success. On every `Err(...)` it returns early **without** notifying.
  - `commands::trigger_upload_now` (Task 17) wraps the error in `Err(e.to_string())` but never calls the notifier.
  - Tray `upload_now` handler (Task 19) does `let _ = state.upload.upload(&cfg).await;` — errors are silently dropped.
  - Shortcut handler (Task 20) does the same silent drop.
- **Why it violates the plan/spec:** The spec's "Notifications and Errors" section lists 15 user-facing messages including `ClipboardEmpty`, `ConfigInvalid`, `UploadFailed`, `SshBinariesMissing`, etc. Users will see **none** of these because neither `UploadService` nor its callers emit notifications on failure.
- **Suggested fix:** Either make `UploadService::upload` emit a `Message` for every `Err` variant before returning, or have each caller (`commands.rs`, `tray.rs`, `shortcut.rs`) match on the error and call `notifier.notify(...)`.

#### 2. Tauri 2 global-shortcut API misuse (`gs.on_shortcut` does not exist)
- **WI ref:** Task 20
- **Evidence:** Task 20 uses:
  ```rust
  let gs = app.global_shortcut();
  gs.on_shortcut(accelerator, move |_app, _shortcut, event| { ... })?;
  ```
- **Why it violates the plan:** `on_shortcut` is a **Tauri v1** API. In Tauri 2 + `tauri-plugin-global-shortcut`, the pattern is:
  1. Register a single handler via `tauri_plugin_global_shortcut::Builder::new().with_handler(...).build()` during plugin init.
  2. Use `gs.register(accelerator)` to bind a shortcut string to that handler.
- **Suggested fix:** Rewrite Task 20 to use the Builder handler pattern, storing the accelerator in managed state so it can be unregistered / re-registered on config save.

#### 3. `create-tauri-app` CLI flag is wrong
- **WI ref:** Task 1, Step 1
- **Evidence:** Command shown is `pnpm create tauri-app clipship-app --template svelte-ts --manager pnpm`.
- **Why it violates the plan:** `create-tauri-app` accepts `--package-manager` (or `-m`), **not** `--manager`. The command will fail with an "unknown option" error and block the entire plan.
- **Suggested fix:** Change to `pnpm create tauri-app@latest clipship-app --template svelte-ts --package-manager pnpm`.

#### 4. chrono millisecond format specifier is incorrect
- **WI ref:** Task 10, `clipboard/image.rs`
- **Evidence:** `chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f")`
- **Why it violates the plan:** In `chrono`, the subsecond format is `%.3f` (three decimal digits). `%3f` is not a recognized specifier and will be emitted literally (or with unpredictable padding), meaning temp filenames will contain the literal text `3f` instead of milliseconds.
- **Suggested fix:** Change to `%.3f`.

---

### High

#### 5. Tray status helpers are defined but never called
- **WI ref:** Task 19 (definition) vs Task 17 / Task 19 (call sites)
- **Evidence:**
  - `tray::set_last_uploaded_enabled` and `tray::set_status` are defined in Task 19 Step 2.
  - The "Decision" paragraph says these will be called from `commands::trigger_upload_now` and the tray `upload_now` handler.
  - Neither Task 17 nor Task 19 actually contains those calls.
- **Impact:** The tray menu will permanently show "Idle" and "Copy last uploaded path" will stay disabled forever, violating the spec's "Current upload status" and "Copy last uploaded path" requirements.
- **Suggested fix:** Add `tray::set_status(&app, "Uploading…")` before `upload()` and `set_last_uploaded_enabled(...)` / `set_status(...)` after it returns in both `commands.rs` and the tray handler.

#### 6. Svelte 5 runes may not compile with default template
- **WI ref:** Task 18
- **Evidence:** Components use `$props()` and `$bindable()`, which are **Svelte 5** runes. The `svelte-ts` template produced by `create-tauri-app` historically ships Svelte 4 unless explicitly updated.
- **Impact:** If the template resolves to Svelte 4, the frontend will fail to compile immediately.
- **Suggested fix:** Either explicitly pin Svelte 5 in `package.json` after scaffolding, or use legacy `export let cfg` props syntax with a compatibility note.

#### 7. `async fn` in trait without `#[async_trait]` will not compile
- **WI ref:** Task 7, `ssh/availability.rs`
- **Evidence:** `trait FallbackExt { async fn unwrap_or_else_fallback(...); }` is defined without `#[async_trait::async_trait]`.
- **Impact:** Compilation failure on stable Rust. The plan even notes this is "fiddly" but still emits broken code.
- **Suggested fix:** Collapse the trait to a plain async helper function as the plan's own note suggests: "collapse to two inline `Command::new(...).output().await` calls."

#### 8. ssh/scp lazy availability check is specified but never wired into handlers
- **WI ref:** Task 21 Step 3 vs Task 17 / Task 19 / Task 20
- **Evidence:** Task 21 shows a code snippet for checking availability inside handlers, but the actual handler implementations in Task 17, Task 19, and Task 20 do **not** contain this check.
- **Impact:** If `ssh`/`scp` is missing, the app will attempt to spawn commands and surface opaque OS errors instead of the actionable `SshBinariesMissing` notification required by the spec.
- **Suggested fix:** Insert the availability check at the top of `trigger_upload_now`, `test_connection` (command), and the tray/shortcut upload paths.

#### 9. macOS private-key permission warning is missing
- **WI ref:** Spec "Configuration" section vs Plan Task 3
- **Evidence:** Spec states: "On macOS, warn when the private key file appears to be readable by group or others". Task 3's `private_key_path` validator only checks `Path::new(p).is_file()`.
- **Impact:** Users with loose key permissions will get silent OpenSSH failures instead of a clear warning.
- **Suggested fix:** Add a macOS-only `#[cfg(target_os = "macos")]` permission check in the validator (or in the settings UI) that returns a warning-level message.

#### 10. Windows `ExitStatusExt` usage in test fakes is unhandled
- **WI ref:** Task 7, `ssh/runner.rs` test fakes
- **Evidence:** `ok_outcome()` and `fail_outcome()` use `std::os::unix::process::ExitStatusExt`. The plan comments: "For Windows-host builds, replace with a cross-platform path... Gate `fakes` with `#[cfg(all(test, unix))]` if Windows unit tests are run."
- **Impact:** Gating `fakes` with `#[cfg(all(test, unix))]` means **no unit tests can run on Windows at all**, which contradicts the cross-platform goal.
- **Suggested fix:** Use `std::process::Command::new("cmd").arg("/C").arg(format!("exit {}", code)).status()` on Windows to synthesize an `ExitStatus`, or use a thin wrapper enum around `(bool, String)` in tests.

---

### Medium

#### 11. `clipboard-rs` `ImageData::to_png()` API assumption may be wrong
- **WI ref:** Task 8, `clipboard/adapter.rs`
- **Evidence:** Code does `img.to_png()` then `png.get_bytes()`. Depending on the exact `clipboard-rs` v0.2 API, `to_png()` may return `Result<Vec<u8>, _>` rather than an object with `.get_bytes()`.
- **Impact:** Clipboard image uploads will fail to compile or panic at runtime.
- **Suggested fix:** Verify the exact `clipboard-rs` v0.2 signature before writing the adapter; have a fallback branch that returns `ClipboardContent::Empty` if PNG conversion fails.

#### 12. `tauri-plugin-dialog` `blocking_show()` may not exist in Tauri 2
- **WI ref:** Task 19, tray quit handler
- **Evidence:** `app_clone.dialog().message(...).blocking_show()` is used. Tauri 2's dialog plugin API is predominantly async (`show()` returning a `Promise<bool>` / `Future<bool>`).
- **Impact:** Compile error or runtime panic if `blocking_show` is unavailable.
- **Suggested fix:** Use an async `show()` inside the spawned task and `.await` the boolean result.

#### 13. `TrayIconBuilder::with_id` API uncertainty
- **WI ref:** Task 19
- **Evidence:** `TrayIconBuilder::with_id("clipship-tray")` is used. In Tauri 2, the typical entry point is `TrayIconBuilder::new(app)` followed by `.id("clipship-tray")` or similar.
- **Impact:** Minor compile-time adjustment likely needed.
- **Suggested fix:** Verify against the installed `tauri` 2.x docs; use `TrayIconBuilder::new(app).id(...)` if `with_id` does not exist.

#### 14. Test-connection failures do not surface notifications
- **WI ref:** Task 15, Task 17, Task 19
- **Evidence:** `test_connection::run` returns `Err(...)` but never notifies. The callers (`commands.rs` and tray handler) also do not notify on test-connection errors.
- **Impact:** User clicks "Test connection" and sees nothing in the UI if it fails (unless the settings window manually parses the returned error string).
- **Suggested fix:** Pipe test-connection errors through the notifier as `Message::MkdirFailed` / a generic test-failure message, or ensure the UI surfaces the returned error in `StatusArea`.

#### 15. `UploadService` does not notify on `ClipboardWrite` failure
- **WI ref:** Task 13
- **Evidence:** When `clipboard.write_text` fails, the function returns `UploadError::ClipboardWrite` early, skipping the final `notifier.notify(...)` call.
- **Impact:** User gets no notification that the upload succeeded but the clipboard could not be updated.
- **Suggested fix:** Emit `Message::ClipboardWriteFailed` explicitly in the `Err(UploadError::ClipboardWrite)` path before returning.

---

### Low / Plan Quality

#### 16. Plan uses interactive `create-tauri-app` prompts instead of flags
- **WI ref:** Task 1
- **Evidence:** Plan says "When prompted..." rather than passing `--name clipship --window-title Clipship --identifier dev.clipship.app`.
- **Impact:** Slightly less deterministic; agentic execution is slower with interactive prompts.
- **Suggested fix:** Add the non-interactive flags to the scaffold command.

#### 17. Two "Decision" paragraphs in Task 19 and Task 21 are not reflected in the accompanying code
- **WI ref:** Task 19 Step 2, Task 21 Step 3
- **Evidence:** Both tasks contain architectural decisions that are absent from the actual code blocks shown in those tasks (and in downstream tasks).
- **Impact:** Increases the chance that the implementing agent misses the decision and ships broken behavior.
- **Suggested fix:** Update the actual code blocks in Task 17, Task 19, and Task 20 to include the decided behavior (tray status updates, availability checks, etc.).

---

## Plan Gaps Summary

| Spec requirement | Plan coverage | Gap |
|---|---|---|
| All 15 notification types surfaced to user | Partial | **Critical**: Error paths never call the notifier. |
| Global shortcut registration (Tauri 2) | Present | **Critical**: Uses v1 API (`on_shortcut`) that does not exist in Tauri 2. |
| macOS private-key permission warning | Missing | **High**: Spec requires it; plan validators do not implement it. |
| Tray shows upload status & last-path | Partial | **High**: Helpers defined but never called in upload handlers. |
| Lazy ssh/scp availability check | Partial | **High**: Specified in Task 21 but omitted from actual handler code. |
| Test connection shows errors | Partial | **Medium**: Errors returned but never notified or shown in UI code. |
| Clipboard write-back failure notification | Partial | **Medium**: `Message::ClipboardWriteFailed` defined but never emitted. |
| Tauri app scaffolding | Present | **Critical**: CLI flag `--manager pnpm` is invalid. |

---

## Test Coverage Gaps

| Missing test | Why it matters |
|---|---|
| Error-path notification tests | No test verifies that `UploadFailed`, `ClipboardEmpty`, `ConfigInvalid`, etc. produce a `Message`. |
| Tray status update tests | No test verifies `set_status` or `set_last_uploaded_enabled` are called. |
| Global shortcut re-registration test | No test verifies that saving config unregisters the old shortcut and registers the new one. |
| macOS key-permission warning test | Not implemented in plan, so no test exists. |
| Windows `ssh.exe` / `scp.exe` argv test | Only one Windows path test exists; no test for the binary-name selection logic on Windows. |
| Clipboard image-to-PNG round-trip with real bytes | Tests use `b"fake png data"` but do not verify actual `clipboard-rs` integration. |

---

## Notes / Risks

1. **Frontend framework version risk:** The plan assumes Svelte 5 (`$props`, `$bindable`) but does not pin it in `package.json`. If `create-tauri-app` resolves to Svelte 4, Task 18 will fail to compile.
2. **Tauri plugin API drift:** Tasks 19 (tray menu), 20 (global shortcut), and 21 (dialog) use APIs that changed significantly between Tauri 1 and Tauri 2. Because the plan targets Tauri 2, several code snippets are likely slightly off and will require doc-driven fixes during implementation.
3. **Cross-platform test fakes:** The plan punts Windows `ExitStatus` synthesis to a hand-wavy comment. Without a concrete cross-platform fake, the entire integration-test suite (`UploadService`, `test_connection`) cannot run on Windows.
4. **Spec drift on ControlMaster:** The spec notes ControlMaster as a future optimization. The plan correctly leaves it out (Task 6 comment), so this is **not** a gap.
5. **No `version` migration path:** The spec says "handled by the migration path" for unversioned config, but the plan only rejects it with `UnversionedOrUnsupported`. There is no actual migration code. For v1 this is acceptable (there is no earlier version to migrate from), but the spec wording implies a migration hook should exist.

---

## Bottom Line

The plan is **structurally sound** and covers the spec's scope well, but it contains **multiple critical execution blockers**:

1. **Compilation blockers:** Wrong CLI flag, invalid Tauri 2 APIs, invalid `async fn` in trait, invalid chrono format string.
2. **Behavioral blockers:** Missing error notifications, missing tray status updates, missing ssh/scp availability checks.
3. **Spec omissions:** Missing macOS private-key permission warning.

**Recommendation:** Do **not** execute the plan verbatim. Fix the Critical and High findings above (especially the compilation errors and the notification wiring) before starting implementation, or expect significant rework after the first few tasks fail.
