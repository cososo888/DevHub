# Security Policy

## Supported versions

| Version | Platform | Supported |
| --- | --- | --- |
| 0.1.x | macOS Apple Silicon | Yes |
| Windows | Not implemented | No |

## Reporting a vulnerability

Please use GitHub's **private vulnerability reporting** in the repository Security tab. Do not open a public issue for a vulnerability involving:

- arbitrary command execution
- injection through search, package IDs or paths
- privilege escalation
- credential or token exposure
- destructive file operations
- bypassing install/uninstall confirmation

Include:

- affected version/commit
- macOS version
- reproduction steps
- expected and actual behavior
- relevant logs with secrets removed

## Scope

DevHub V2 executes Homebrew commands using fixed argv. It does not accept arbitrary commands from the frontend.

The current macOS DMG is ad-hoc signed and not notarized. Gatekeeper warnings are expected; this is not a vulnerability by itself.

Windows is not supported yet. Do not publish a Windows security claim until the winget implementation and NSIS build have been reviewed.
