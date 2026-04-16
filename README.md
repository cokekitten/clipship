# Clipship

[![GitHub](https://img.shields.io/badge/GitHub-cokekitten%2Fclipship-blue?logo=github)](https://github.com/cokekitten/clipship)

A lightweight, cross-platform clipboard uploader that turns screenshots, copied images, or files into instantly shareable paths — so you can paste them straight into your favorite coding CLI and let the LLM see what you see.

---

## Why Clipship?

Modern coding assistants in the terminal (like Claude Code, Cursor CLI, Aider, etc.) are incredibly powerful, but they live in a text-only world. When you want an LLM to look at a screenshot, a diagram, or a local file, the friction is real: you have to save it somewhere, find the path, and then type it out.

**Clipship removes that friction.**

1. Copy or screenshot anything.
2. Press a global hotkey.
3. Paste the resulting path into your CLI.

The LLM now has direct access to the image or file.

### Works with Remote Development, Too

If your coding CLI runs on a remote machine via SSH, Clipship has you covered. It can automatically detect the remote OS, provision a temporary directory, and upload the file over SCP so the path you paste is valid on the remote side.

---

## How It Works

| Step | Action | Result |
|------|--------|--------|
| 1 | **Copy** an image, a file, or take a screenshot. | Clipboard captures the content. |
| 2 | **Press the global shortcut** (`CmdOrCtrl+Shift+U` by default). | Clipship uploads it to a stable location. |
| 3 | **Paste** the returned path into your coding CLI. | The LLM reads the file directly. |

### Example Flow

```
You: [takes a screenshot of an error message]
You: [presses Ctrl+Shift+U]
Clipship: /tmp/clipship/20260416-161145-377-efqgjx-clipboard.png
You: "look at this error /tmp/clipship/20260416-161145-377-efqgjx-clipboard.png"
LLM: "I see the issue..."
```

---

## Features

- **One-hotkey upload** — screenshots, copied images, or files.
- **Local mode** — writes to a local temp directory, perfect for local CLI workflows.
- **SSH mode** — connects to a remote host, auto-detects the OS (macOS / Linux), and uploads to the remote temp directory over SCP.
- **Auto-cleanup** — optionally deletes files older than 7 days every hour.
- **Global shortcut with double-tap guard** — press once or require a quick double-tap to avoid accidents.
- **Tray icon** — runs quietly in the background; right-click for quick actions.

---

## Installation

> Pre-built binaries will be available on the [Releases](https://github.com/cokekitten/clipship/releases) page soon.

### Build from Source

Requirements: [Node.js](https://nodejs.org/), [pnpm](https://pnpm.io/), [Rust](https://rustup.rs/)

```bash
# Clone the repository
git clone https://github.com/cokekitten/clipship.git
cd clipship

# Install frontend dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build a production binary
pnpm tauri build
```

---

## Usage

1. Launch Clipship. It will sit in your system tray.
2. Open the settings window from the tray icon.
3. Choose your mode:
   - **Local** — files are saved to your local temp directory.
   - **SSH** — enter your host, username, and private key; Clipship will test the connection, detect the remote OS, and pick the right temp directory for you.
4. Set your preferred global shortcut (default is `CmdOrCtrl+Shift+U`).
5. Close the settings window — Clipship keeps running in the background.
6. Whenever you copy an image or file, press the shortcut and paste the path into your CLI.

---

## Supported Platforms

| Platform | Local Mode | SSH Remote Upload |
|----------|------------|-------------------|
| Windows  | ✅         | ✅ (client side)  |
| macOS    | ✅         | ✅ (local & remote) |
| Linux    | ✅         | ✅ (local & remote) |

> **Note:** SSH auto-configuration currently supports macOS and Linux remotes. For Windows SSH servers, please configure the destination directory manually.

---

## License

MIT

---

Made with ☕ by [cokekitten](https://github.com/cokekitten)
