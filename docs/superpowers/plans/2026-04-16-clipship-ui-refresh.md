# Clipship UI Refresh + Shortcut Recorder + Double-Tap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework the settings window using Tailwind + shadcn-svelte, replace the free-text shortcut field with a key-capture recorder, and add an optional "double-tap to trigger" modifier for the global shortcut.

**Architecture:** Frontend gets a component library (shadcn-svelte on bits-ui + Tailwind v3). A dedicated `ShortcutRecorder` Svelte component captures `keydown` events and emits a Tauri-compatible accelerator string. Backend grows a `shortcut_double_tap` flag on `Config`; `shortcut::register` consults a pure `should_fire` helper to decide whether each `Pressed` transition triggers the upload, keyed off a per-app `Mutex<Option<Instant>>`.

**Tech Stack:** Svelte 5, TypeScript, Vite, Tailwind CSS v3, shadcn-svelte + bits-ui, Vitest (new), Rust (Tauri 2), existing `tauri-plugin-global-shortcut`.

---

## Dependency setup and file layout

Files created or modified across this plan:

**New frontend files**
- `tailwind.config.js` — Tailwind scan config
- `postcss.config.js` — PostCSS pipeline
- `components.json` — shadcn-svelte config
- `src/app.css` — Tailwind entry stylesheet
- `src/lib/utils.ts` — `cn()` helper
- `src/lib/components/ui/button/` (and `input`, `label`, `switch`, `card`) — shadcn-svelte generated
- `src/components/ShortcutRecorder.svelte` — new capture component
- `src/lib/shortcut/map.ts` — KeyboardEvent → Tauri accelerator mapping
- `src/lib/shortcut/map.test.ts` — unit tests (Vitest)
- `vitest.config.ts` — Vitest config

**Modified frontend files**
- `package.json` — dev deps, `test` script
- `src/main.ts` — import `app.css`
- `src/App.svelte` — shadcn rewrite + `shortcut_double_tap` init + recorder wiring
- `src/components/SshSection.svelte`, `DestinationSection.svelte`, `ShortcutSection.svelte`, `StatusArea.svelte` — rewrite with shadcn primitives
- `src/lib/types.ts` — add `shortcut_double_tap`
- `src/lib/bridge.ts` — no function signature changes, but exercised in testing

**New backend files**
- `src-tauri/src/shortcut/mod.rs` (promotes current `shortcut.rs` to a module)
- `src-tauri/src/shortcut/detect.rs` — pure `should_fire` helper + unit tests

**Modified backend files**
- `src-tauri/Cargo.toml` — no new deps expected
- `src-tauri/src/lib.rs` — module path
- `src-tauri/src/app_state.rs` — add `last_shortcut_press`
- `src-tauri/src/config/mod.rs` — add `shortcut_double_tap` field + default + round-trip test

---

## Task 1: Install Tailwind + PostCSS + base CSS

**Files:**
- Modify: `package.json`
- Create: `tailwind.config.js`
- Create: `postcss.config.js`
- Create: `src/app.css`
- Modify: `src/main.ts`

- [ ] **Step 1: Install dev dependencies**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm add -D tailwindcss@^3.4.0 postcss autoprefixer tailwindcss-animate
```

Expected: `package.json` and `pnpm-lock.yaml` updated; no errors.

- [ ] **Step 2: Create `tailwind.config.js`**

```js
import animate from "tailwindcss-animate";

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{svelte,ts,js}"],
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
    },
  },
  plugins: [animate],
};
```

- [ ] **Step 3: Create `postcss.config.js`**

```js
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 4: Create `src/app.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 47.4% 11.2%;
    --muted: 210 40% 96.1%;
    --muted-foreground: 215.4 16.3% 46.9%;
    --popover: 0 0% 100%;
    --popover-foreground: 222.2 47.4% 11.2%;
    --border: 214.3 31.8% 91.4%;
    --input: 214.3 31.8% 91.4%;
    --card: 0 0% 100%;
    --card-foreground: 222.2 47.4% 11.2%;
    --primary: 222.2 47.4% 11.2%;
    --primary-foreground: 210 40% 98%;
    --secondary: 210 40% 96.1%;
    --secondary-foreground: 222.2 47.4% 11.2%;
    --accent: 210 40% 96.1%;
    --accent-foreground: 222.2 47.4% 11.2%;
    --destructive: 0 100% 50%;
    --destructive-foreground: 210 40% 98%;
    --ring: 215 20.2% 65.1%;
    --radius: 0.5rem;
  }
  body {
    @apply bg-background text-foreground;
  }
}
```

- [ ] **Step 5: Wire the stylesheet into `src/main.ts`**

Open `src/main.ts`. Add this line as the FIRST import:

```ts
import "./app.css";
```

- [ ] **Step 6: Run dev build to verify**

```bash
pnpm vite build
```

Expected: build succeeds, Tailwind utilities are scanned, no PostCSS errors.

- [ ] **Step 7: Commit**

```bash
git add package.json pnpm-lock.yaml tailwind.config.js postcss.config.js src/app.css src/main.ts
git commit -m "Wire Tailwind CSS v3 into the Vite build"
```

---

## Task 2: Add shadcn-svelte and generate primitives

**Files:**
- Modify: `package.json`
- Create: `components.json`
- Create: `src/lib/utils.ts`
- Create: `src/lib/components/ui/button/*`, `input/*`, `label/*`, `switch/*`, `card/*`

- [ ] **Step 1: Install runtime dependencies used by shadcn-svelte**

```bash
pnpm add bits-ui clsx tailwind-merge
```

- [ ] **Step 2: Create `src/lib/utils.ts`**

```ts
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 3: Create `components.json` at project root**

```json
{
  "$schema": "https://shadcn-svelte.com/schema.json",
  "style": "default",
  "tailwind": {
    "config": "tailwind.config.js",
    "css": "src/app.css",
    "baseColor": "neutral"
  },
  "aliases": {
    "components": "$lib/components",
    "utils": "$lib/utils",
    "ui": "$lib/components/ui",
    "hooks": "$lib/hooks",
    "lib": "$lib"
  },
  "typescript": true,
  "registry": "https://shadcn-svelte.com/registry"
}
```

- [ ] **Step 4: Ensure `$lib` alias is set up**

Open `svelte.config.js`. If the file does not exist, create it; if it does, verify it contains an alias mapping `$lib` → `./src/lib`. Expected final content:

```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  kit: {},
  compilerOptions: {},
};
```

Then open `vite.config.ts` (or `vite.config.js`) and add the alias — example final content:

```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "node:path";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, "./src/lib"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
```

Keep any existing Tauri-specific server settings that were present before; only merge in the alias.

- [ ] **Step 5: Generate the five primitives**

```bash
pnpm dlx shadcn-svelte@latest add button input label switch card
```

If the CLI asks for confirmations, answer yes. Expected: files created under `src/lib/components/ui/`.

- [ ] **Step 6: Sanity import check**

Create a throwaway assertion in `src/main.ts` — TEMPORARILY add this block after the app mount to make sure imports resolve at build time:

```ts
import { Button } from "$lib/components/ui/button";
console.log(!!Button);
```

Run `pnpm vite build` — build must succeed. Then REMOVE the two lines above before committing.

- [ ] **Step 7: Commit**

```bash
git add package.json pnpm-lock.yaml components.json svelte.config.js vite.config.ts src/lib/utils.ts src/lib/components
git commit -m "Bring in shadcn-svelte primitives (button/input/label/switch/card)"
```

---

## Task 3: Rewrite settings sections with shadcn primitives

**Files:**
- Modify: `src/components/SshSection.svelte`
- Modify: `src/components/DestinationSection.svelte`
- Modify: `src/components/ShortcutSection.svelte`
- Modify: `src/components/StatusArea.svelte`
- Modify: `src/App.svelte`

This is a visual refactor only — no feature changes. Each file below shows the full new content.

- [ ] **Step 1: Rewrite `SshSection.svelte`**

```svelte
<script lang="ts">
  import type { Config } from "../lib/types";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";

  let { cfg = $bindable() }: { cfg: Config } = $props();

  async function pickKey() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Select SSH private key",
    });
    if (typeof picked === "string") cfg.private_key_path = picked;
  }
</script>

<Card>
  <CardHeader>
    <CardTitle>SSH</CardTitle>
  </CardHeader>
  <CardContent class="grid gap-4">
    <div class="grid gap-1.5">
      <Label for="ssh-host">Host</Label>
      <Input id="ssh-host" bind:value={cfg.host} placeholder="example.com or ::1" />
    </div>
    <div class="grid grid-cols-2 gap-4">
      <div class="grid gap-1.5">
        <Label for="ssh-port">Port</Label>
        <Input id="ssh-port" type="number" bind:value={cfg.port} min="1" max="65535" />
      </div>
      <div class="grid gap-1.5">
        <Label for="ssh-user">Username</Label>
        <Input id="ssh-user" bind:value={cfg.username} />
      </div>
    </div>
    <div class="grid gap-1.5">
      <Label for="ssh-key">Private key</Label>
      <div class="flex gap-2">
        <Input id="ssh-key" class="flex-1" bind:value={cfg.private_key_path} readonly />
        <Button variant="secondary" type="button" onclick={pickKey}>Browse…</Button>
      </div>
    </div>
  </CardContent>
</Card>
```

- [ ] **Step 2: Rewrite `DestinationSection.svelte`**

Read the current file first to preserve the field set (remote_dir). Then replace with:

```svelte
<script lang="ts">
  import type { Config } from "../lib/types";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";

  let { cfg = $bindable() }: { cfg: Config } = $props();
</script>

<Card>
  <CardHeader>
    <CardTitle>Destination</CardTitle>
  </CardHeader>
  <CardContent class="grid gap-4">
    <div class="grid gap-1.5">
      <Label for="remote-dir">Remote directory</Label>
      <Input id="remote-dir" bind:value={cfg.remote_dir} placeholder="/var/www/uploads" />
    </div>
  </CardContent>
</Card>
```

- [ ] **Step 3: Rewrite `ShortcutSection.svelte` (recorder wired in a later task)**

For now keep a basic text input so builds still work — the recorder arrives in Task 10.

```svelte
<script lang="ts">
  import type { Config } from "../lib/types";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";

  let { cfg = $bindable() }: { cfg: Config } = $props();
</script>

<Card>
  <CardHeader>
    <CardTitle>Shortcut</CardTitle>
  </CardHeader>
  <CardContent class="grid gap-4">
    <div class="grid gap-1.5">
      <Label for="shortcut">Global shortcut</Label>
      <Input id="shortcut" bind:value={cfg.shortcut} placeholder="CmdOrCtrl+Shift+U" />
    </div>
  </CardContent>
</Card>
```

- [ ] **Step 4: Rewrite `StatusArea.svelte`**

```svelte
<script lang="ts">
  import type { Status } from "../lib/types";
  let { status }: { status: Status } = $props();
</script>

{#if status.kind !== "idle" || status.message}
  <div
    class={[
      "rounded-md border p-3 text-sm",
      status.kind === "error" ? "border-destructive/30 bg-destructive/5 text-destructive" : "",
      status.kind === "ok" ? "border-emerald-200 bg-emerald-50 text-emerald-800" : "",
      status.kind === "idle" ? "border-border bg-muted text-muted-foreground" : "",
    ].join(" ")}
  >
    <div class="font-medium">{status.message}</div>
    {#if status.detail}
      <pre class="mt-2 whitespace-pre-wrap text-xs">{status.detail}</pre>
    {/if}
  </div>
{/if}
```

- [ ] **Step 5: Rewrite `App.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import type { Config, Status } from "./lib/types";
  import { loadConfig, saveConfig, testConnection, getAutostart, setAutostart, defaultPrivateKey } from "./lib/bridge";
  import SshSection from "./components/SshSection.svelte";
  import DestinationSection from "./components/DestinationSection.svelte";
  import ShortcutSection from "./components/ShortcutSection.svelte";
  import StatusArea from "./components/StatusArea.svelte";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card";
  import { Switch } from "$lib/components/ui/switch";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";

  let cfg: Config = $state({
    version: 1,
    host: "",
    port: 22,
    username: "",
    private_key_path: "",
    remote_dir: "",
    shortcut: "CmdOrCtrl+Shift+U",
    shortcut_double_tap: false,
  });

  let status: Status = $state({ kind: "idle", message: "" });
  let autostart: boolean = $state(false);

  onMount(async () => {
    try {
      cfg = await loadConfig();
    } catch (e) {
      status = { kind: "error", message: "Failed to load configuration", detail: String(e) };
    }
    if (!cfg.private_key_path) {
      try {
        const k = await defaultPrivateKey();
        if (k) cfg.private_key_path = k;
      } catch (_) {}
    }
    try {
      autostart = await getAutostart();
    } catch (_) {}
  });

  async function onSave() {
    try {
      const result = await saveConfig(cfg);
      status = result.warnings.length
        ? { kind: "ok", message: "Saved with warnings.", detail: result.warnings.join("\n") }
        : { kind: "ok", message: "Saved." };
    } catch (e) {
      status = { kind: "error", message: "Save failed", detail: String(e) };
    }
  }

  async function onTest() {
    status = { kind: "idle", message: "Testing…" };
    try {
      await testConnection(cfg);
      status = { kind: "ok", message: "Connection OK." };
    } catch (e) {
      status = { kind: "error", message: "Test failed", detail: String(e) };
    }
  }

  async function onAutostartChange(v: boolean) {
    autostart = v;
    try {
      await setAutostart(autostart);
    } catch (e) {
      status = { kind: "error", message: "Failed to update autostart", detail: String(e) };
      autostart = !autostart;
    }
  }
</script>

<main class="mx-auto flex max-w-2xl flex-col gap-4 p-6">
  <h1 class="text-xl font-semibold">Clipship</h1>
  <SshSection bind:cfg />
  <DestinationSection bind:cfg />
  <ShortcutSection bind:cfg />
  <Card>
    <CardHeader>
      <CardTitle>System</CardTitle>
    </CardHeader>
    <CardContent class="flex items-center justify-between">
      <Label for="autostart" class="flex flex-col gap-1">
        <span>Launch at login</span>
        <span class="text-xs font-normal text-muted-foreground">
          Start Clipship automatically when you sign in.
        </span>
      </Label>
      <Switch id="autostart" checked={autostart} onCheckedChange={onAutostartChange} />
    </CardContent>
  </Card>
  <div class="flex gap-2">
    <Button onclick={onSave}>Save</Button>
    <Button variant="secondary" onclick={onTest}>Test connection</Button>
  </div>
  <StatusArea {status} />
</main>
```

- [ ] **Step 6: Build to verify**

```bash
pnpm vite build
```

Expected: build succeeds with no TS/Svelte errors. If a Switch API property differs from `onCheckedChange` in the installed shadcn-svelte version, match whatever the generated component exports (read `src/lib/components/ui/switch/switch.svelte` and adapt).

- [ ] **Step 7: Run `pnpm tauri dev` and sanity-check the window**

```bash
pnpm tauri dev
```

Confirm visually: four cards, buttons render, Save + Test + Switch work. Stop with Ctrl+C after checking.

- [ ] **Step 8: Commit**

```bash
git add src/App.svelte src/components
git commit -m "Restyle settings panel with shadcn-svelte primitives"
```

---

## Task 4: Extend Config with `shortcut_double_tap` (backend, TDD)

**Files:**
- Modify: `src-tauri/src/config/mod.rs`

- [ ] **Step 1: Write the failing tests**

Open `src-tauri/src/config/mod.rs`. Find the `#[cfg(test)] mod tests` block. Add two tests at the end of that block (before its closing brace):

```rust
    #[test]
    fn deserializes_without_shortcut_double_tap_field() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"version":1,"host":"h","port":22,"username":"u","private_key_path":"","remote_dir":"/r","shortcut":"CmdOrCtrl+Shift+U"}"#,
        )
        .unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.shortcut_double_tap, false);
    }

    #[test]
    fn round_trip_preserves_shortcut_double_tap() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.host = "h".into();
        cfg.shortcut_double_tap = true;
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert!(back.shortcut_double_tap);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd C:/Users/CokeKitten/dev/clipship/src-tauri
cargo test config::mod --lib -- --nocapture
```

Expected: compile error (`no field shortcut_double_tap`).

- [ ] **Step 3: Add the field and serde default**

In `src-tauri/src/config/mod.rs`, change the `Config` struct to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key_path: String,
    pub remote_dir: String,
    pub shortcut: String,
    #[serde(default)]
    pub shortcut_double_tap: bool,
}
```

And add the default in `impl Default for Config`:

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            host: String::new(),
            port: 22,
            username: String::new(),
            private_key_path: String::new(),
            remote_dir: String::new(),
            shortcut: "CmdOrCtrl+Shift+U".into(),
            shortcut_double_tap: false,
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --lib
```

Expected: all existing tests still pass + both new ones pass.

- [ ] **Step 5: Commit**

```bash
cd C:/Users/CokeKitten/dev/clipship
git add src-tauri/src/config/mod.rs
git commit -m "Add shortcut_double_tap config field with serde default"
```

---

## Task 5: Thread `shortcut_double_tap` through the frontend types

**Files:**
- Modify: `src/lib/types.ts`

- [ ] **Step 1: Add the field**

Open `src/lib/types.ts` and change the `Config` interface to:

```ts
export interface Config {
  version: 1;
  host: string;
  port: number;
  username: string;
  private_key_path: string;
  remote_dir: string;
  shortcut: string;
  shortcut_double_tap: boolean;
}
```

- [ ] **Step 2: Build to verify**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm vite build
```

Expected: build passes. `App.svelte` already initializes the field from Task 3.

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "Mirror shortcut_double_tap in frontend Config type"
```

---

## Task 6: Extract pure `should_fire` helper for double-tap

**Files:**
- Create: `src-tauri/src/shortcut/mod.rs` (moved from `shortcut.rs`)
- Create: `src-tauri/src/shortcut/detect.rs`
- Modify: `src-tauri/src/lib.rs` (no-op — `pub mod shortcut;` stays)

- [ ] **Step 1: Promote `shortcut.rs` to a module directory**

```bash
cd C:/Users/CokeKitten/dev/clipship/src-tauri/src
mkdir shortcut
git mv shortcut.rs shortcut/mod.rs
```

Verify `src-tauri/src/lib.rs` still compiles — it declares `pub mod shortcut;` which now resolves to `shortcut/mod.rs`.

- [ ] **Step 2: Create `src-tauri/src/shortcut/detect.rs` with failing tests**

```rust
use std::time::{Duration, Instant};

/// Decide whether a `Pressed` transition should fire the upload and what the
/// new `last_press` state should be.
///
/// * `prev` — timestamp of the previous `Pressed` transition, if any.
/// * `now` — timestamp of the current `Pressed` transition.
/// * `window` — max gap between two presses that counts as a double-tap.
///
/// Returns `(fire, next_state)`. If `fire` is true, a double-tap was detected
/// and the caller should trigger the upload AND reset state to `None`. If
/// `fire` is false, the caller stores `next_state` as the new `last_press`.
pub fn should_fire(
    prev: Option<Instant>,
    now: Instant,
    window: Duration,
) -> (bool, Option<Instant>) {
    match prev {
        Some(p) if now.duration_since(p) <= window => (true, None),
        _ => (false, Some(now)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_press_only_stores_timestamp() {
        let now = Instant::now();
        let (fire, next) = should_fire(None, now, Duration::from_millis(400));
        assert!(!fire);
        assert_eq!(next, Some(now));
    }

    #[test]
    fn second_press_inside_window_fires_and_clears() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(399);
        let (fire, next) = should_fire(Some(t0), t1, Duration::from_millis(400));
        assert!(fire);
        assert_eq!(next, None);
    }

    #[test]
    fn second_press_at_exact_window_boundary_fires() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(400);
        let (fire, next) = should_fire(Some(t0), t1, Duration::from_millis(400));
        assert!(fire);
        assert_eq!(next, None);
    }

    #[test]
    fn second_press_outside_window_rearms_timestamp() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(401);
        let (fire, next) = should_fire(Some(t0), t1, Duration::from_millis(400));
        assert!(!fire);
        assert_eq!(next, Some(t1));
    }
}
```

- [ ] **Step 3: Expose the helper from the module**

Open `src-tauri/src/shortcut/mod.rs` and add at the top (below the imports block):

```rust
pub mod detect;
```

- [ ] **Step 4: Run tests to verify**

```bash
cd C:/Users/CokeKitten/dev/clipship/src-tauri
cargo test --lib shortcut::detect
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
cd C:/Users/CokeKitten/dev/clipship
git add src-tauri/src/shortcut
git commit -m "Add pure should_fire helper for double-tap detection"
```

---

## Task 7: Wire double-tap into the shortcut handler

**Files:**
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/shortcut/mod.rs`

- [ ] **Step 1: Add `last_shortcut_press` to `AppState`**

Read `src-tauri/src/app_state.rs`. Add this field to the `AppState` struct:

```rust
    pub last_shortcut_press: std::sync::Mutex<Option<std::time::Instant>>,
```

Initialize it to `Mutex::new(None)` in the `AppState::build` (or equivalent) constructor that returns `AppState`.

- [ ] **Step 2: Rewrite `register()` to use the helper**

Open `src-tauri/src/shortcut/mod.rs`. Replace the handler closure inside `register` with:

```rust
use crate::shortcut::detect::should_fire;
use std::time::{Duration, Instant};

const DOUBLE_TAP_WINDOW_MS: u64 = 400;

// ... inside register(), replace the on_shortcut handler:
    let app_for_handler = app.clone();
    match gs.on_shortcut(accelerator, move |_app, _shortcut, event| {
        if event.state() != ShortcutState::Pressed {
            return;
        }
        let state = app_for_handler.state::<AppState>();
        let cfg = match crate::config::load(&state.config_path) {
            Ok(c) => c,
            Err(_) => {
                // If config is unreadable, fall back to immediate fire.
                tauri::async_runtime::spawn(run_shortcut_upload(app_for_handler.clone()));
                return;
            }
        };

        if !cfg.shortcut_double_tap {
            tauri::async_runtime::spawn(run_shortcut_upload(app_for_handler.clone()));
            return;
        }

        let mut guard = state.last_shortcut_press.lock().unwrap();
        let (fire, next) = should_fire(
            *guard,
            Instant::now(),
            Duration::from_millis(DOUBLE_TAP_WINDOW_MS),
        );
        *guard = next;
        drop(guard);

        if fire {
            tauri::async_runtime::spawn(run_shortcut_upload(app_for_handler.clone()));
        }
    }) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            app.state::<AppState>()
                .upload
                .notifier
                .notify(Message::ShortcutRegistrationFailed(msg.clone()));
            Err(msg)
        }
    }
```

Remove the stand-alone `Instant` / `Duration` imports if they're already present in the file — do not duplicate.

- [ ] **Step 3: Build and run existing tests**

```bash
cd C:/Users/CokeKitten/dev/clipship/src-tauri
cargo build
cargo test --lib
```

Expected: compile succeeds; all tests still pass.

- [ ] **Step 4: Manual verification**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm tauri dev
```

In the app, toggle `shortcut_double_tap` through `config.json` directly (the UI switch lands in Task 10) or skip this manual check until Task 10 lands.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/app_state.rs src-tauri/src/shortcut/mod.rs
git commit -m "Gate shortcut upload on double-tap when enabled"
```

---

## Task 8: Install Vitest and add the shortcut key map

**Files:**
- Modify: `package.json`
- Create: `vitest.config.ts`
- Create: `src/lib/shortcut/map.ts`
- Create: `src/lib/shortcut/map.test.ts`

- [ ] **Step 1: Install Vitest**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm add -D vitest
```

- [ ] **Step 2: Add `test` script**

Open `package.json`. Add to `scripts`:

```json
    "test": "vitest run",
    "test:watch": "vitest"
```

- [ ] **Step 3: Create `vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
```

- [ ] **Step 4: Write the failing tests first**

Create `src/lib/shortcut/map.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { eventToAccelerator } from "./map";

function mk(
  code: string,
  key: string,
  mods: Partial<{ ctrl: boolean; alt: boolean; shift: boolean; meta: boolean }> = {},
) {
  return {
    code,
    key,
    ctrlKey: !!mods.ctrl,
    altKey: !!mods.alt,
    shiftKey: !!mods.shift,
    metaKey: !!mods.meta,
  } as KeyboardEvent;
}

describe("eventToAccelerator", () => {
  it("maps Ctrl+Shift+U", () => {
    expect(eventToAccelerator(mk("KeyU", "U", { ctrl: true, shift: true }))).toBe(
      "CmdOrCtrl+Shift+U",
    );
  });

  it("orders modifiers CmdOrCtrl → Alt → Shift → Super", () => {
    expect(
      eventToAccelerator(mk("KeyD", "D", { ctrl: true, alt: true, shift: true, meta: true })),
    ).toBe("CmdOrCtrl+Alt+Shift+Super+D");
  });

  it("maps digit keys by physical code", () => {
    expect(eventToAccelerator(mk("Digit5", "%", { ctrl: true, shift: true }))).toBe(
      "CmdOrCtrl+Shift+5",
    );
  });

  it("maps F-keys", () => {
    expect(eventToAccelerator(mk("F12", "F12", { ctrl: true }))).toBe("CmdOrCtrl+F12");
  });

  it("maps named keys (Space, Enter, Tab, Backspace, Delete, Escape, arrows)", () => {
    expect(eventToAccelerator(mk("Space", " ", { ctrl: true }))).toBe("CmdOrCtrl+Space");
    expect(eventToAccelerator(mk("Enter", "Enter", { alt: true }))).toBe("Alt+Enter");
    expect(eventToAccelerator(mk("ArrowLeft", "ArrowLeft", { ctrl: true }))).toBe(
      "CmdOrCtrl+Left",
    );
    expect(eventToAccelerator(mk("ArrowRight", "ArrowRight", { ctrl: true }))).toBe(
      "CmdOrCtrl+Right",
    );
    expect(eventToAccelerator(mk("ArrowUp", "ArrowUp", { ctrl: true }))).toBe("CmdOrCtrl+Up");
    expect(eventToAccelerator(mk("ArrowDown", "ArrowDown", { ctrl: true }))).toBe(
      "CmdOrCtrl+Down",
    );
  });

  it("rejects combos without any modifier", () => {
    expect(eventToAccelerator(mk("KeyU", "U"))).toBeNull();
  });

  it("rejects modifier-only press", () => {
    expect(eventToAccelerator(mk("ControlLeft", "Control", { ctrl: true }))).toBeNull();
    expect(eventToAccelerator(mk("ShiftRight", "Shift", { shift: true }))).toBeNull();
  });

  it("returns null for unknown codes", () => {
    expect(eventToAccelerator(mk("IntlRo", "", { ctrl: true }))).toBeNull();
  });
});
```

- [ ] **Step 5: Run to confirm failure**

```bash
pnpm test
```

Expected: fails because `./map` does not exist yet.

- [ ] **Step 6: Implement `src/lib/shortcut/map.ts`**

```ts
const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
  "OSLeft",
  "OSRight",
]);

const NAMED: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Escape: "Escape",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backquote: "`",
};

function codeToKey(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return `Num${code.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (NAMED[code]) return NAMED[code];
  return null;
}

export function eventToAccelerator(e: KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("CmdOrCtrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");

  if (parts.length === 0) return null;

  const key = codeToKey(e.code);
  if (!key) return null;

  parts.push(key);
  return parts.join("+");
}
```

- [ ] **Step 7: Run tests to verify they pass**

```bash
pnpm test
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add package.json pnpm-lock.yaml vitest.config.ts src/lib/shortcut
git commit -m "Map KeyboardEvent to Tauri accelerator tokens"
```

---

## Task 9: `ShortcutRecorder.svelte` component

**Files:**
- Create: `src/components/ShortcutRecorder.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { eventToAccelerator } from "../lib/shortcut/map";

  let {
    value,
    onChange,
  }: { value: string; onChange: (v: string) => void } = $props();

  let recording = $state(false);

  function start() {
    recording = true;
  }

  function stop() {
    recording = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape" && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      stop();
      return;
    }

    const accel = eventToAccelerator(e);
    if (!accel) {
      return; // wait for a valid combo — modifier-only / no-modifier events ignored
    }
    onChange(accel);
    stop();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex items-center gap-2">
  {#if recording}
    <div
      class="inline-flex h-9 items-center rounded-md border border-amber-400 bg-amber-50 px-3 text-sm text-amber-900"
    >
      Press shortcut… <span class="ml-2 text-xs text-amber-700">(Esc to cancel)</span>
    </div>
    <Button variant="secondary" type="button" onclick={stop}>Cancel</Button>
  {:else}
    <code class="inline-flex h-9 items-center rounded-md border bg-muted px-3 text-sm">
      {value || "(unset)"}
    </code>
    <Button variant="secondary" type="button" onclick={start}>Change</Button>
  {/if}
</div>
```

- [ ] **Step 2: Build**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm vite build
```

Expected: build passes.

- [ ] **Step 3: Commit**

```bash
git add src/components/ShortcutRecorder.svelte
git commit -m "Add ShortcutRecorder component with key-capture UX"
```

---

## Task 10: Wire the recorder and double-tap switch into `ShortcutSection`

**Files:**
- Modify: `src/components/ShortcutSection.svelte`

- [ ] **Step 1: Rewrite the section**

```svelte
<script lang="ts">
  import type { Config } from "../lib/types";
  import ShortcutRecorder from "./ShortcutRecorder.svelte";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card";
  import { Label } from "$lib/components/ui/label";
  import { Switch } from "$lib/components/ui/switch";

  let { cfg = $bindable() }: { cfg: Config } = $props();

  function onAccel(v: string) {
    cfg.shortcut = v;
  }

  function onDoubleTapChange(v: boolean) {
    cfg.shortcut_double_tap = v;
  }
</script>

<Card>
  <CardHeader>
    <CardTitle>Shortcut</CardTitle>
  </CardHeader>
  <CardContent class="grid gap-4">
    <div class="grid gap-1.5">
      <Label>Global shortcut</Label>
      <ShortcutRecorder value={cfg.shortcut} onChange={onAccel} />
    </div>
    <div class="flex items-center justify-between">
      <Label for="double-tap" class="flex flex-col gap-1">
        <span>Require double-tap to trigger</span>
        <span class="text-xs font-normal text-muted-foreground">
          Press the shortcut twice within 400&nbsp;ms to upload.
        </span>
      </Label>
      <Switch
        id="double-tap"
        checked={cfg.shortcut_double_tap}
        onCheckedChange={onDoubleTapChange}
      />
    </div>
  </CardContent>
</Card>
```

- [ ] **Step 2: Build**

```bash
pnpm vite build
```

Expected: build passes.

- [ ] **Step 3: Manual verification**

```bash
pnpm tauri dev
```

In the running app:
1. Click `Change`, press `Ctrl+Shift+U` — confirm the chip updates and the recorder exits.
2. Click `Change`, press `Esc` — confirm the value is unchanged.
3. Click `Change`, then click `Cancel` — same.
4. Toggle the double-tap Switch on, click `Save`, then try the shortcut:
   - Single press — no upload (tray stays Idle).
   - Two presses within 400 ms — upload runs.
   - Alternative tap variants from spec (e.g., hold `U`, double-tap `Ctrl`) — also trigger.
5. Toggle the switch off, Save, single-press the shortcut — upload fires as before.

- [ ] **Step 4: Commit**

```bash
git add src/components/ShortcutSection.svelte
git commit -m "Wire shortcut recorder and double-tap switch into settings"
```

---

## Task 11: Final regression sweep

- [ ] **Step 1: Run all tests**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm test
cd src-tauri
cargo test --lib
```

Expected: all green.

- [ ] **Step 2: Full build**

```bash
cd C:/Users/CokeKitten/dev/clipship
pnpm tauri build
```

Expected: successful build of the MSI artifact (or at minimum, `pnpm vite build` + `cargo build --release` if the MSI bundler isn't wanted locally).

- [ ] **Step 3: Final manual smoke**

Launch the built binary (or `pnpm tauri dev`), verify:
- Settings window looks polished (cards, spacing, buttons).
- All four sections render and save.
- Private key auto-fills if `~/.ssh` has one.
- Shortcut recorder + double-tap both work end-to-end.

- [ ] **Step 4: (Optional) merge back**

This plan was developed on `main`. If a separate branch/worktree was used, open a PR or fast-forward merge.
