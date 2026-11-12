# Contributing to DevHub V2

Thanks for helping improve DevHub V2.

## Before you start

- Read `README.md` and `docs/development.md`.
- For Windows work, use the `windows-handoff` branch and read `docs/WINDOWS-TEAMMATE-GUIDE.md`.
- Do not modify the legacy DevHub V1 repository.
- Do not test real installs/uninstalls on a machine with important data.

## Branching

- `main` is the stable integration branch.
- `windows-handoff` is for Windows work.
- Create a feature branch from the appropriate base:
  ```bash
  git switch main
  git pull
  git switch -c feature/your-change
  ```

## Development

```bash
npm ci
npm run check
npm run test:ui
cargo test --manifest-path src-tauri/Cargo.toml
```

Keep changes focused. UI changes should preserve the circular grid, shared morph card, minimal detail, light/dark themes and glob placeholder behavior.

## Pull requests

- Explain what changed and why.
- Include test results and screenshots for UI changes.
- State whether any real install/uninstall was performed. Normally it should not be.
- Do not include secrets, tokens, local database files or personal paths.
- Do not force push or rewrite published history.
- Windows changes must state that Windows is still incomplete until the NSIS build and manual checklist pass.

## Commit style

Use clear prefixes when useful:

```text
feat: ...
fix: ...
docs: ...
style: ...
refactor: ...
chore: ...
```

## Code of conduct

By participating, you agree to follow `CODE_OF_CONDUCT.md`.
