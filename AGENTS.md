# DevHub V2 working agreement

- Work in this V2 repository. Do not modify the legacy DevHub project.
- Current product scope: macOS only. Windows work is paused.
- Efficiency is a product requirement: concise, useful information and fewer unnecessary steps. Avoid filler, repeated introductions, decorative status text and unsolicited automatic reordering.
- Software details describe the software's purpose and concrete use cases. Keep package-manager policy, permission/impact confirmation and troubleshooting beside the relevant action, not inside the software introduction.
- Keep technical logs available on demand; do not hide uncertainty or claim success from a process exit alone.
- Preserve colorful brand artwork in light and dark modes. Prefer bundled local assets; label generic fallbacks honestly rather than implying an official logo.
- Reuse the existing orb grid and detail components. Keep theme, keyboard access and scrolling usable.
- Check the worktree before editing and preserve user changes. Build the updated app for handoff. Testing is user-led: do not automatically run automated tests or operate the native app for validation unless the user asks. State honestly which checks were actually performed.
- Do not run real host software installs/uninstalls, edit PATH/Shell/system configuration or delete user data without separate explicit authorization. No remote push without user instruction.

- Software introductions open only on explicit click, never hover or focus. Keep the summary and trigger stable during disclosure; detailed text includes concrete purpose and actual use conditions, not package-manager boilerplate.

- Scope operation errors to the exact software/package kind that produced them; never show one software's failure in another detail card.

- Detail software artwork must scroll with its header/content; a separate morph-animation copy must be invisible once the card is open.
- Treat an existing non-Homebrew .app as external ownership: do not overwrite, adopt or expose package-manager uninstall for it.

- Global search is a separate page entered from the magnifier. Keep its query/results independent of library categories, installation filters and scroll position; preserve both when navigating back.
- Use macOS UserNotifications with actual consent/status checks. Do not assume a native notification submission means the banner was displayed. Notification tests are triggered by the user from the notification settings in the sidebar guide.
- Installed software uses a subtle blue halo, without multiple hard circular outlines.
