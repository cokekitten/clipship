# Clipship Design

## Summary

Clipship is a cross-platform desktop utility for macOS and Windows. It runs in the menu bar or system tray, provides a configuration window, registers a global shortcut, and uploads the first file or image in the clipboard to a configured SSH destination.

After a successful upload, Clipship writes the remote absolute path back to the system clipboard. It does not generate public URLs.

## Goals

- Provide a Tauri desktop app with a tray/menu-bar presence and configuration UI.
- Let users configure SSH host, port, username, private key path, remote directory, and global shortcut.
- Use the system `ssh` and `scp` commands for the first version.
- Support clipboard file upload and clipboard image upload.
- Rename every upload with a timestamp prefix to avoid remote filename collisions.
- Automatically create the configured remote directory before uploading.
- Copy the final remote absolute path to the clipboard after upload succeeds.
- Show system notifications for success and actionable failure states.

## Non-Goals

- Password-based SSH authentication.
- Public URL generation.
- Directory upload.
- Multi-file upload in the first version.
- Built-in SSH/SFTP protocol implementation.
- File synchronization, history, or remote file management.

## Product Shape

Clipship is a Tauri app with:

- A tray/menu-bar icon.
- A configuration window.
- A global shortcut that triggers upload from the clipboard.
- Optional startup behavior in a later phase.

The app should be usable without keeping the configuration window open. The tray/menu-bar menu should expose at least:

- Upload clipboard now.
- Open settings.
- Test connection.
- Quit.

## Configuration

Clipship stores local app configuration only. It does not store private key contents.

Configuration should be persisted under the Tauri app config directory, for example `app_config_dir/clipship/config.json`. Future migrations should version this file rather than moving configuration into the project directory or a custom hidden folder.

Version 1 must write an explicit `"version": 1` field into `config.json`. A config file missing this field should be treated as corrupt or pre-v1 and handled by the migration path rather than silently accepted, so future schema changes have a reliable signal to key off.

Required settings:

- `host`: remote SSH host or IP.
- `port`: SSH port, default `22`.
- `username`: SSH username.
- `private_key_path`: local private key path, selected from `~/.ssh` or picked manually.
- `remote_dir`: remote absolute directory, such as `/home/ubuntu/uploads`.
- `shortcut`: global shortcut, such as `CmdOrCtrl+Shift+U`.

Configuration save behavior:

- Validate field presence and basic format.
- Restrict `host` to hostname, IPv4, or IPv6 literal formats accepted by OpenSSH, and reject values starting with `-`.
- Restrict `username` to `A-Z`, `a-z`, `0-9`, `_`, `.`, and `-`, and reject values starting with `-`.
- Require `remote_dir` to be an absolute POSIX path starting with `/`.
- Restrict `remote_dir` to `A-Z`, `a-z`, `0-9`, `_`, `.`, `/`, and `-`.
- Reject `remote_dir` values containing `..` path segments.
- Reject whitespace, quotes, backticks, dollar signs, backslashes, control characters, and newlines in `remote_dir`.
- Version 1 intentionally rejects non-ASCII characters in `remote_dir`. Safely quoting arbitrary Unicode for the remote shell is out of scope for v1, so users must configure an ASCII-only remote directory. The settings UI must surface this limitation clearly so a path like `/home/user/上传/` is rejected with an explanation, not treated as a bug.
- Require `port` to be an integer between 1 and 65535.
- Require `private_key_path` to point to an existing local file.
- On macOS, warn when the private key file appears to be readable by group or others, because OpenSSH may reject loose key permissions.
- Do not automatically open an SSH connection when saving.

The settings UI should provide a separate "Test connection" action.

## SSH and SCP Strategy

Version 1 uses system commands:

- `ssh` creates the remote directory with `mkdir -p`.
- `scp` uploads the prepared local file.

Equivalent command model:

```bash
ssh -p <port> -i <key> \
  -o BatchMode=yes \
  -o StrictHostKeyChecking=accept-new \
  -o ConnectTimeout=10 \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  <user>@<host> "mkdir -p '<remote_dir>'"

scp -P <port> -i <key> \
  -o BatchMode=yes \
  -o StrictHostKeyChecking=accept-new \
  -o ConnectTimeout=10 \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  <local_file> <user>@<host>:<remote_part_path>

ssh -p <port> -i <key> \
  -o BatchMode=yes \
  -o StrictHostKeyChecking=accept-new \
  -o ConnectTimeout=10 \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  <user>@<host> "mv -n '<remote_part_path>' '<remote_path>'"
```

The implementation should invoke commands without local shell interpolation. Arguments should be passed as process argument arrays to avoid local quoting issues. Remote shell commands still pass through the remote shell, so `remote_dir` and generated remote filenames must be restricted to the safe character set defined in the configuration and file naming sections.

SSH commands must use `BatchMode=yes` so passphrase prompts, password prompts, and other interactive authentication requests fail quickly instead of blocking the UI. Users who use passphrase-protected keys must unlock them through `ssh-agent` before using Clipship. `StrictHostKeyChecking=accept-new` should accept first-time host keys while still failing if a known host key changes.

On startup or before upload, Clipship should detect whether `ssh` and `scp` are available. If either is missing, it should show an actionable error. On Windows, this likely means enabling or installing OpenSSH Client.

By default, Clipship uses modern OpenSSH `scp` behavior. OpenSSH 9 and newer may use SFTP under the hood. Legacy servers that require old SCP protocol may need a future `-O` compatibility option.

When `host` is an IPv6 literal such as `::1`, `ssh` accepts `user@host` as-is, but `scp`'s remote target `user@host:path` requires the host to be wrapped in square brackets, for example `user@[::1]:/remote/path`. The command builder must detect IPv6 literal hosts and add the brackets only for the `scp` target, not for the `ssh` `user@host` argument. Hostnames and IPv4 literals must not be bracketed.

The final remote `mv -n` is a guard: if the destination already exists — which should be vanishingly rare given the millisecond + random suffix in the filename — `mv -n` refuses to overwrite and Clipship surfaces the failure to the user rather than silently replacing an existing remote file. The `.part` file remains for the user to inspect or clean up.

Future optimization (not required for v1): a single upload currently opens three independent SSH connections (`mkdir`, `scp`, `mv`). This can be collapsed via `ControlMaster=auto` with a short-lived `ControlPath` in the app's temp directory, so the second and third connections reuse the first handshake. This is worth doing once v1 is validated, because for small files the three handshakes dominate the wall-clock cost.

## Clipboard Handling

When the global shortcut fires, Clipship inspects the clipboard and uploads the first supported content:

1. If the clipboard contains one or more files, upload the first file.
2. Otherwise, if the clipboard contains image data, write it to a temporary PNG and upload that file.
3. Otherwise, show a notification that the clipboard has no uploadable file or image.

Only regular files are supported in the first version. Directories should produce a clear unsupported-content notification.

Clipboard image temporary files:

- Use a temporary app-specific directory.
- Name the local temporary source with enough uniqueness to avoid local conflicts.
- Delete the temporary file after a successful upload.
- Keep the temporary file after a failed upload to aid debugging.

## File Naming

All uploads are renamed before sending to the remote server. This prevents remote collisions at the source.

Format:

```text
YYYYMMDD-HHMMSS-SSS-<rand6>-<safe-original-name.ext>
```

Examples:

```text
20260416-183012-427-a8f31c-report.pdf
20260416-183012-427-a8f31c-clipboard.png
```

Rules:

- Preserve the original extension for file uploads.
- Use `clipboard.png` for image clipboard uploads.
- Sanitize unsafe filename characters.
- Convert whitespace runs to `-`.
- Avoid path separators and control characters.
- If sanitization produces an empty stem, use `file`.
- Restrict generated remote filenames to `A-Z`, `a-z`, `0-9`, `_`, `.`, and `-`.
- Add both milliseconds and a six-character random suffix to prevent collisions from repeated shortcut triggers in the same second.

## Upload Flow

Shortcut-triggered upload:

1. Load and validate saved configuration.
2. Detect supported clipboard content.
3. Prepare local upload source.
4. Build timestamped safe remote filename.
5. Build final remote path as `remote_dir + "/" + remote_filename`.
6. Snapshot the current clipboard state so the app can detect later user clipboard changes. The snapshot granularity is defined as follows:
   - For file clipboard content, store the ordered list of absolute file paths. The clipboard "matches" later if and only if it still contains files and the list of paths is identical.
   - For image clipboard content, store the SHA-256 of the raw image bytes captured at snapshot time. The clipboard "matches" later if and only if it still contains image data with the same SHA-256.
   - Any transition between clipboard kinds (e.g., file → text, image → file) counts as "changed", even if the user happens to put something equivalent back.
7. Run `ssh` to create `remote_dir` with `mkdir -p`.
8. Run `ssh rm -f '<remote_part_path>'` to clear any leftover residue at this exact `.part` path. Because the `.part` path inherits the millisecond + random suffix of this specific upload, this only ever removes Clipship's own prior residue at that precise name, never unrelated user files.
9. Run `scp` to upload local source to `remote_path + ".part"`.
10. Run `ssh mv -n` to atomically move the `.part` file to the final remote path. If `mv -n` refuses because the destination already exists, treat the upload as failed and preserve the `.part` file for inspection.
11. If the clipboard still matches the original snapshot, write final remote absolute path to the system clipboard.
12. If the clipboard changed during upload, leave it unchanged and expose the final remote path in the notification/status area for manual copying.
13. Show a success notification.
14. If the source was a temporary clipboard image, delete it after success.

If any step fails, Clipship should leave the previous clipboard content unchanged unless the upload already succeeded and clipboard replacement is the failing step.

When `scp` or the final `mv -n` fails, the failure notification must include the full `.part` path so the user can clean it up manually if they choose. Clipship does not otherwise sweep stale `.part` files on the remote, because enumerating and deleting in `remote_dir` is a broader capability than v1 needs and increases blast radius for any path-validation bug.

Only one upload may run at a time. If the user triggers the shortcut or tray action while an upload is already in flight, Clipship should not start another `ssh` or `scp` process. It should show a short busy notification and keep the current upload state unchanged.

If the user chooses Quit from the tray menu while an upload is in flight, Clipship must prompt for confirmation. If confirmed, it terminates the child `ssh`/`scp` process, logs that the remote `.part` file is likely left behind, and exits. Version 1 does not offer a separate in-flight "cancel" action — the only way to abort an upload is to confirm Quit.

The tray/menu-bar state should show when an upload is in progress. Exact byte-level progress is not required for version 1 because parsing `scp` progress output is platform-sensitive, but users must have visible feedback during long uploads.

## Test Connection Flow

The settings UI exposes "Test connection".

Test connection should:

1. Validate current form values.
2. Check `ssh` availability.
3. Run the same remote directory creation command used before upload.
4. Create a probe file named `.clipship-probe-<rand>` in `remote_dir`.
5. Remove the probe file.
6. Report success or show the command failure summary.

The probe file validates that the target directory exists and that the configured user has write and delete permissions. It should be removed even if a later test step fails.

## Notifications and Errors

User-facing notifications:

- Upload succeeded and copied remote path.
- Clipboard has no uploadable file or image.
- Clipboard content is a directory, which is unsupported.
- Missing or invalid configuration.
- Missing `ssh` or `scp`.
- Remote directory creation failed.
- Upload failed.
- Upload is already in progress.
- Upload succeeded but the clipboard changed, so the remote path was not copied automatically.
- Upload failed and a remote `.part` file may remain at a given absolute path.
- Upload aborted because the final destination already exists (the `mv -n` guard).
- Quit confirmation while an upload is in flight.
- Clipboard write-back failed.

Detailed command stderr should be available in the UI or a diagnostics area, but notifications should stay short.

## UI Structure

Settings window:

- SSH section: host, port, username, private key path picker.
- Destination section: remote absolute directory.
- Shortcut section: editable global shortcut capture.
- Actions: save, test connection.
- Status area: latest operation result and detailed error text when relevant.

Tray/menu-bar menu:

- Upload clipboard now.
- Open settings.
- Test connection.
- Copy last uploaded path, when available.
- Current upload status.
- Quit.

## Platform Notes

macOS:

- Use Tauri tray/menu-bar support.
- Clipboard file URLs and image data must be handled through platform-compatible APIs.
- Global shortcut should use `CmdOrCtrl` semantics in settings display where possible.
- Closing the settings window should hide the window, not quit the app.
- The app should behave as a menu-bar utility. If the final Tauri configuration uses `LSUIElement=true` or accessory activation policy, settings window focus and visibility must be manually verified.
- First-run UX must handle missing permissions for global shortcuts, clipboard access, and notifications. If a required permission is missing, the app should show actionable guidance instead of failing silently.

Windows:

- Detect `ssh.exe` and `scp.exe`.
- Support private keys under `%USERPROFILE%\.ssh`.
- Clipboard file list and image data should use Windows-compatible APIs exposed through Tauri or Rust crates.
- Paths passed to `scp` must be local Windows paths as process arguments, not shell strings.

## Testing Strategy

Unit tests:

- Configuration validation.
- Remote path construction.
- Filename sanitization.
- Timestamped rename behavior.
- Command argument construction for `ssh` and `scp`.
- Remote directory validation and unsafe character rejection.
- Probe command construction for test connection.
- In-flight upload guard behavior.
- Clipboard snapshot comparison behavior.
- Clipboard content classification where logic is separable from platform APIs.

Integration-style tests with fakes:

- Upload flow calls directory creation before file upload.
- Upload flow uploads to `.part` and renames to the final path after transfer.
- Upload flow aborts and surfaces the `.part` path when the final destination already exists and `mv -n` refuses to overwrite.
- IPv6 literal hosts produce bracketed `scp` targets (`user@[::1]:/path`) and unbracketed `ssh` `user@host` arguments.
- Configuration loader rejects `config.json` files missing the `"version"` field and routes them through the migration path.
- Clipboard snapshot comparison treats files-by-path-list and images-by-SHA-256, and treats any kind transition as changed.
- Success writes the remote path to clipboard.
- Success does not overwrite clipboard when clipboard content changed during upload.
- Failed upload does not overwrite clipboard.
- Repeated trigger during upload does not start a second upload.
- Temporary image file is deleted on success and retained on failure.

Manual verification:

- macOS shortcut triggers upload while settings window is closed.
- Windows shortcut triggers upload while settings window is closed.
- File clipboard upload works.
- Image clipboard upload works.
- Missing `scp`/`ssh` produces actionable error.
- Test connection creates missing remote directory and verifies write/delete permissions.
- Passphrase-protected keys fail quickly unless unlocked in `ssh-agent`.
- First-time host key handling works with `StrictHostKeyChecking=accept-new`.
- Large uploads show visible in-progress state.
- macOS screenshots are tested when the clipboard may contain both file URL and image data; file content remains the first priority.
- Quit from the tray while an upload is in flight prompts for confirmation and leaves the remote `.part` path in the failure log.
- An attempted upload whose final filename happens to already exist on the remote is aborted cleanly by `mv -n`, with the `.part` file preserved.

## Implementation Phases

1. Scaffold Tauri app and project structure.
2. Implement configuration model, validation, and persistence.
3. Implement command availability checks and SSH/SCP command builders.
4. Implement upload orchestration with test doubles.
5. Implement clipboard adapters for file and image content.
6. Implement settings UI and tray/menu-bar menu.
7. Register global shortcut and connect it to upload flow.
8. Add notifications and diagnostics.
9. Package and manually verify macOS and Windows behavior.
