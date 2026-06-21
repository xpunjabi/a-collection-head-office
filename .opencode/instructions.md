# Local Machine / CI Policy

This PC is a protected authoring machine for code editing only.

## Rules
- Never install anything on this local PC unless the owner explicitly asks.
- Never run package manager or installer commands locally.
  Examples: cargo add, cargo install, rustup, npm/pnpm/yarn install, pip/uv/poetry add, winget/choco/scoop/brew/apt, docker setup, daemon/service setup.
- Never add or change project dependencies without owner approval.
  This includes Cargo.toml, package.json, lockfiles, and any tool/runtime dependency changes.
- Never run local build, compile, test, migration, packaging, or long-running service commands by default.
- Use GitHub Actions / remote CI for builds, tests, compile checks, packaging, and repeated validation.
- If a task truly requires a local action or a new dependency, stop and ask the owner first.

## Default behavior
- Local PC = read/edit/write code only
- GitHub Actions / CI = run/build/test/package
