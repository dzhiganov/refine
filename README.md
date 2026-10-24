# Refine

A desktop app (Windows + macOS) for getting better at writing in another language, one real text
at a time. Paste what you wrote; Refine fixes only what's actually wrong, explains each fix in plain
words, and suggests a few ways to sound more natural. Over time it notices the mistakes that keep
coming back and helps you get rid of them.

- **Minimal fixes.** Grammar errors and anything that could be misunderstood. If it's understandable
  and correct, it stays as you wrote it.
- **Three blocks.** Fixed text (with an inline diff), mistakes (each with a short explanation and a
  category), suggestions (more natural phrasing, or a synonym for variety). Suggestions are never
  counted as mistakes.
- **Tones.** Slack, GitHub, formal email, friends, or your own. The tone tells the model where the
  text is going, so `pls`/`thx` are fine in Slack and a register mistake in a formal email.
- **Your model.** OpenRouter (pick any model), OpenAI, or a local model via Ollama / LM Studio.
  Keys live in the macOS Keychain / Windows Credential Manager.
- **Strict JSON everywhere.** Model answers are schema-constrained and validated; every stored
  record, the stats and the one-click export share one versioned, typed contract.
- **Progress that means something.** Errors per 100 words with a significance test before calling
  anything a trend, a watch list of recurring patterns (max 3), a "fix your own sentence" review on
  an expanding schedule, and tracking of suggestions you actually start using. Computed on your
  machine; the model only writes a short summary. Why it works this way:
  [docs/research/stats-system.md](docs/research/stats-system.md) and [docs/decisions.md](docs/decisions.md).

## Download

Website: [refine-writing.vercel.app](https://refine-writing.vercel.app). Installers for macOS
(universal `.dmg`) and Windows (`.exe` / `.msi`) are on the
[latest release](https://github.com/dzhiganov/refine/releases/latest). They aren't signed with a paid
certificate yet; the release notes say how to open them the first time.

To publish a new version: bump `version` in `package.json`, `src-tauri/tauri.conf.json` and
`Cargo.toml`, then push a tag (`git tag v0.2.0 && git push origin v0.2.0`). CI builds both
installers and attaches them to a new release.

## Website

`site/` is the landing page: static HTML, CSS and a little JS, deployed to Vercel
(`cd site && vercel deploy --prod`). Download buttons point at
`releases/latest/download/Refine-macOS.dmg` (and the Windows files), which CI attaches to every
release, so the site never needs a redeploy for a new version; it reads the version number from the
GitHub API. Demo media is recorded from the real UI: `pnpm dev`, then `node scripts/media.mjs`.

## Stack

| Part | What |
|---|---|
| `crates/core` | Rust: LLM client, prompts + JSON schema, validation, SQLite store, stats engine, export. No UI code. |
| `src-tauri` | Tauri 2 shell: commands over `refine_core::app::Core`, the idle worker, window config. |
| `src` | React 19 + TypeScript + CSS modules + Motion. Design tokens from Mind. |
| `src/bindings` | TypeScript types generated from the Rust types (`pnpm bindings`). Don't edit by hand. |

## Develop

```bash
pnpm install
pnpm dev                       # UI in a browser at http://localhost:1420 with a mock backend
cargo test -p refine-core      # core tests (writes src/bindings too)
pnpm typecheck && pnpm test    # UI checks
```

**Windows** (from WSL): `scripts/win.sh run` mirrors the source to `%USERPROFILE%\dev\refine` and runs
the real app there against the WSL dev server (start `pnpm dev` first). `scripts/win.sh bundle`
builds the installers. Needs Rust (MSVC) and Visual Studio Build Tools on Windows.

**macOS:** `pnpm tauri dev`, or `pnpm tauri build` for a `.dmg`. CI builds a universal `.dmg` on every
push to `main` (`.github/workflows/build.yml`). The build is ad-hoc signed, not notarized: on first
launch macOS blocks it; go to System Settings → Privacy & Security and click Open Anyway (or run
`xattr -dr com.apple.quarantine /Applications/Refine.app`).

**Linux dev box without sudo:** the Tauri system libraries can live in a local sysroot; see
`docs/dev-linux.md`.

**Local model (LM Studio):** start the server (`lms server start`), load a model (`lms load
qwen/qwen3.6-35b-a3b --context-length 16384`), then in Refine pick Settings → Model → LM Studio and the
model. With **Thinking: Off** (the default) a check takes 2–10 s on an RTX 4070 Ti SUPER; **Low**
catches more subtle collocations but takes ~50 s. See "Local models" in `docs/decisions.md`.

**Real-model smoke test:**

```bash
REFINE_API_KEY=sk-or-... REFINE_MODEL=openai/gpt-5-mini cargo run -p refine-core --example smoke
REFINE_PROVIDER=lmstudio REFINE_MODEL=qwen/qwen3.6-35b-a3b cargo run -p refine-core --example smoke
```

`scripts/e2e-windows.mjs` drives the real Windows app over WebView2's DevTools port (see its header);
`REFINE_DATA_DIR` points the app at a scratch data folder.

Demo data for the browser mock is generated by the real stats engine:
`cargo run -p refine-core --example demo_data`.

## Data

Everything stays local: `refine.sqlite3` in the app data folder (`%APPDATA%\app.refine.desktop` on
Windows, `~/Library/Application Support/app.refine.desktop` on macOS). Export writes
`refine-export-YYYY-MM-DD.json` to Downloads: settings (no keys), tones, every entry, stats and the
latest summary, with `format: "refine.export"` and a `schemaVersion`.
