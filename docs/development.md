# Development Guide

This document is for contributors who want to run, modify or build DevHub V2 locally.

## Requirements

- macOS (the shipped build is Apple Silicon)
- Node.js 20+
- Rust stable
- Xcode Command Line Tools
- Homebrew (only for install/uninstall actions; view-only detection works without it)

Windows development is not ready yet. See `docs/WINDOWS-PORT-REPORT.md` and `docs/WINDOWS-TEAMMATE-GUIDE.md`.

## Setup

```bash
git clone https://github.com/cososo888/DevHub.git
cd DevHub
npm ci
```

Start the complete desktop app:

```bash
npm run tauri -- dev
```

Start only the Vite frontend:

```bash
npm run dev
```

The Tauri dev URL is `http://localhost:1421`.

## Commands

| Command | Purpose |
| --- | --- |
| `npm run dev` | Vite frontend only |
| `npm run tauri -- dev` | Full Tauri app |
| `npm run check` | TypeScript type check |
| `npm run build` | Frontend production build |
| `npm run test:ui` | UI/component tests |
| `npm run tauri -- build --bundles dmg` | macOS DMG |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust tests |

## Architecture

- `src/` — React UI, catalog, search filtering, install task events.
- `src-tauri/` — Tauri commands, Homebrew adapter, detection, bounded process runner, SQLite storage.
- `scripts/` — Node-based catalog/UI tests.
- `docs/` — phase reports, screenshots and Windows handoff material.

The macOS app currently uses Homebrew as the package provider. Windows will use winget; do not add Windows-specific provider code to the macOS path.

## Data and safety

- Detection is independent of DevHub install history.
- Install/uninstall commands use fixed argv, never shell string concatenation.
- The app does not silently request administrator privileges.
- Tests must not perform real installs/uninstalls.
- Do not edit PATH, shell profiles, or remove user data.

## Packaging

```bash
npm run tauri -- build --bundles dmg
```

Output:

```text
src-tauri/target/release/bundle/dmg/DevHub V2_0.1.0_aarch64.dmg
```

The local build is ad-hoc signed and not notarized. SmartScreen/Gatekeeper warnings are expected.

## Windows

The Windows branch is `windows-handoff`. Current status:

- winget provider: not implemented
- Windows detection: not implemented
- NSIS installer: configuration prepared, not built
- 100 winget IDs: candidates only, not verified

Do not claim Windows support until `npm run tauri -- build --bundles nsis` succeeds on Windows x64 and the manual checklist passes.

## Release checklist

- `npm run check`
- `npm run test:ui`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Build DMG
- Verify checksum
- Update `CHANGELOG.md`
- Do not force push
