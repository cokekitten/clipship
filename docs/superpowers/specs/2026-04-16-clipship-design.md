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

Required settings:

- `host`: remote SSH host or IP.
- `port`: SSH port, default `22`.
- `username`: SSH username.
- `private_key_path`: local private key path, selected from `~/.ssh` or picked manually.
- `remote_dir`: remote absolute directory, such as `/home/ubuntu/uploads`.
- `shortcut`: global shortcut, such as `CmdOrCtrl+Shift+U`.

Configuration save behavior:

- Validate field presence and basic format.
- Require `remote_dir` to be an absolute POSIX path starting with `/`.
- Require `port` to be an integer between 1 and 65535.
- Require `private_key_path` to point to an existing local file.
- Do not automatically open an SSH connection when saving.

The settings UI should provide a separate "Test connection" action.

## SSH and SCP Strategy

Version 1 uses system commands:

- `ssh` creates the remote directory with `mkdir -p`.
- `scp` uploads the prepared local file.

Equivalent command model:

```bash
ssh -p <port> -i <key> <user>@<host> "mkdir -p '<remote_dir>'"
scp -P <port> -i <key> <local_file> <user>@<host>:<remote_path>
```

The implementation should invoke commands without shell interpolation whenever possible. Arguments should be passed as process argument arrays to avoid quoting issues. Remote shell commands still require careful escaping for the `mkdir -p` path.

On startup or before upload, Clipship should detect whether `ssh` and `scp` are available. If either is missing, it should show an actionable error. On Windows, this likely means enabling or installing OpenSSH Client.

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
YYYYMMDD-HHMMSS-<safe-original-name.ext>
```

Examples:

```text
20260416-183012-report.pdf
20260416-183012-clipboard.png
```

Rules:

- Preserve the original extension for file uploads.
- Use `clipboard.png` for image clipboard uploads.
- Sanitize unsafe filename characters.
- Convert whitespace runs to `-`.
- Avoid path separators and control characters.
- If sanitization produces an empty stem, use `file`.

## Upload Flow

Shortcut-triggered upload:

1. Load and validate saved configuration.
2. Detect supported clipboard content.
3. Prepare local upload source.
4. Build timestamped safe remote filename.
5. Build final remote path as `remote_dir + "/" + remote_filename`.
6. Run `ssh` to create `remote_dir` with `mkdir -p`.
7. Run `scp` to upload local source to final remote path.
8. On success, write final remote absolute path to the system clipboard.
9. Show a success notification.
10. If the source was a temporary clipboard image, delete it after success.

If any step fails, Clipship should leave the previous clipboard content unchanged unless the upload already succeeded and clipboard replacement is the failing step.

## Test Connection Flow

The settings UI exposes "Test connection".

Test connection should:

1. Validate current form values.
2. Check `ssh` availability.
3. Run the same remote directory creation command used before upload.
4. Report success or show the command failure summary.

It should not upload a test file in the first version.

## Notifications and Errors

User-facing notifications:

- Upload succeeded and copied remote path.
- Clipboard has no uploadable file or image.
- Clipboard content is a directory, which is unsupported.
- Missing or invalid configuration.
- Missing `ssh` or `scp`.
- Remote directory creation failed.
- Upload failed.
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
- Quit.

## Platform Notes

macOS:

- Use Tauri tray/menu-bar support.
- Clipboard file URLs and image data must be handled through platform-compatible APIs.
- Global shortcut should use `CmdOrCtrl` semantics in settings display where possible.

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
- Clipboard content classification where logic is separable from platform APIs.

Integration-style tests with fakes:

- Upload flow calls directory creation before file upload.
- Success writes the remote path to clipboard.
- Failed upload does not overwrite clipboard.
- Temporary image file is deleted on success and retained on failure.

Manual verification:

- macOS shortcut triggers upload while settings window is closed.
- Windows shortcut triggers upload while settings window is closed.
- File clipboard upload works.
- Image clipboard upload works.
- Missing `scp`/`ssh` produces actionable error.
- Test connection creates missing remote directory.

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

