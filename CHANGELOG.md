# Changelog

All notable changes to DevHub V2 are documented here.

## [Unreleased]

### Added

- Windows handoff documentation, winget candidate list and NSIS configuration.
- Beginner-oriented Windows teammate guide.

### Changed

- Search is global when a query is present; the selected category is ignored until the query is cleared.
- Search is case-insensitive.

## [0.1.0] - 2026-11-12

### Added

- macOS Apple Silicon desktop app built with Tauri 2, React 19 and Rust.
- 100 curated Homebrew packages across 8 categories, with 84 formulae and 16 casks.
- Homebrew full-library search with debounce, caching and error/retry states.
- Local environment detection: deep detection for the original 14 tools and generic Homebrew package state for the rest.
- Fixed-command Homebrew install/uninstall with confirmation, bounded argv execution, progress and technical logs.
- Light/dark themes, circular software grid, shared morph detail card and hidden page scrollbar.

### Fixed

- Go post-install false negative caused by `brew list --formula go` exceeding the output limit. Verification now uses exact version queries and exact formula JSON matching.
- Installed ring is more visible, especially in dark mode.
- Search no longer depends on the selected category when a query is present.

### Security

- Install and uninstall actions are confirmed before execution.
- Only Homebrew-managed packages can be uninstalled.
- No silent administrator elevation; no PATH/shell modification.

### Known limitations

- No version selection.
- Database/Redis services are not started automatically.
- macOS DMG is unsigned/ad-hoc and not notarized.
- Windows support is not implemented.
