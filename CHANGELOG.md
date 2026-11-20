# Changelog

All notable changes to DevHub V2 are documented here.

## [Unreleased]

### Seamless macOS window chrome

- Replace the separate system title strip with a transparent overlay, hide its title and keep the native traffic-light buttons.
- Share one full-width glass backdrop across the sidebar and toolbar, with a soft fade into the software grid.
- Keep categories centered over the software column and reserve space beside the window buttons for the compact DevHub icon.
- Allow dragging the empty header area; preserve category, search and appearance controls as ordinary clickable controls.
- Build-only handoff for user testing; no automated tests or native window interaction performed.

### Dock lifetime and install notifications

- On macOS, closing the main window hides it while the app stays in the Dock and current work continues.
- Clicking the Dock icon restores the same window and its state; Dock Quit and Cmd+Q still exit normally.
- Send a backend-owned system notification only after successful installation has been reverified, including while the window is hidden.
- Clicking a completion notification brings DevHub back; system notification settings still control delivery.
- Build-only handoff: no automated tests, window actions or real package install/uninstall performed.


### Existing apps, scrolling details and DevHub branding

- Detect existing curated cask applications in fixed application folders without running them; keep external ownership separate from Homebrew records.
- Stop before downloading/installing over an existing unregistered app and explain same-name app conflicts in Chinese while retaining the original log.
- Keep the real software icon inside the scrolling detail header; hide the separate morph artwork once open.
- Add an original blue/indigo D-and-connection DevHub mark to the upper-left sidebar and replace the bundled application icon assets.
- Build-only handoff for user testing: no automated/native tests or real package install/uninstall performed.


### Package-state errors and softer chrome

- Treat empty nonzero Homebrew version queries as inconclusive and recheck exact formula/cask JSON, rather than blocking uninstalled casks with a generic error.
- Keep raw query diagnostics when rechecking fails; bind action errors to the package and kind that produced them.
- Use a single circled exclamation glyph without an outer button ring.
- Keep the centered glass toolbar within the software column, with a blurred fading lower edge instead of a divider.
- Match detail surfaces to the orb's pale tint, normalize new brand artwork to the original logos' optical size, and place detection metadata directly left of refresh.
- This update is handed off for user testing; no automated or native UI tests and no real software installation/uninstallation were performed.


### Stable introductions and installation filters

- Make the plain exclamation disclosure click-only and keep the summary visible, preventing the hover/resize loop.
- Expand curated software introductions with concrete purpose, use cases and use conditions.
- Replace the three theme controls with one menu for day, night and automatic appearance.
- Add left-side filters for all, installed and uninstalled software, combined with search and categories.
- Preserve the keyword when search auto-collapses; show an explicit clear-keyword action.
- Do not classify pending, stale or failed state queries as proof that software is uninstalled.


### Efficient details and colorful icons

- Remove generic package-manager policy from software introductions and avoid repeating the short description.
- Describe common tools and Memcached by purpose and use cases; replace the other curated catalog templates with concrete one-line descriptions.
- Restore local brand colors, add tinted plates for neutral marks, and remove theme-wide icon inversion.
- Remove decorative development-stage labels and duplicate footer copy; record efficiency-first UI/content rules for ongoing macOS development.
- Validation: 25 UI tests, type check and debug macOS app build passed. Light-mode native grid/details were inspected; no real package install/uninstall was performed.


### macOS UI refinement

- Dismiss search when selecting another control or scrolling outside it; remove the top-left brand block.
- Keep grid order stable after installed state updates.
- Preserve the source orb radius and tinted glass surface when expanding details.
- Add click-to-toggle software information in the scrollable detail card.
- Bundle Simple Icons artwork locally and replace remaining letter placeholders with tool glyphs.
- Automated UI checks and debug app packaging cover these changes; visual/native interaction acceptance remains pending because preview tools were unavailable.

### macOS reliability and usability

- Focus current development on macOS; pause the Windows plan and retain its documents as historical references.
- Hide stale remote results when search keywords change; add a clear-search action and explain category behavior.
- Separate readable failure guidance from technical logs; preserve post-install verification errors.
- Show task stages without fabricated percentages and clarify that Memcached services are not auto-started.
- These changes do not constitute a real Memcached installation/uninstallation acceptance test.


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
