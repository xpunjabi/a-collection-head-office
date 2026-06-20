# Project Rules for AI Assistant

## Development Environment
- This PC is for CODE EDITING ONLY.
- NO tools/software installation on this PC.
- NO compilation, building, or testing locally.
- All builds and testing happen via GitHub Actions.

## Workflow
1. Write/fix code on this PC.
2. Commit and push to GitHub.
3. GitHub Actions automatically builds, tests, and creates releases.
4. Download release artifacts from GitHub Releases page.

## Version Management
- Update version numbers in `src-tauri/Cargo.toml` before tagging.
- Use semantic versioning (v0.x.x).

## Git Commits
- Commit messages should be descriptive in English.
- Push tags for releases (e.g., `v0.4.1`).
