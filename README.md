# A Collection Head Office

A lightweight, local-first, AI-assisted business operating system for small clothing businesses.

Built with **Tauri 2**, **React**, **TypeScript**, **Rust**, and **SQLite**.

## Features

- **Dashboard** — Business overview with stats, charts, and quick actions
- **Catalog** — Product management with CRUD, CSV import/export, image upload
- **Social Hub** — AI-powered content generation for Facebook, Instagram, WhatsApp, X
- **Customers** — Customer profiles, purchase history, order placement
- **Inventory** — Stock tracking, low stock alerts, dead stock audit, best sellers
- **Automation** — Scheduled backup and weekly report generation
- **Reports** — Sales, inventory, and customer reports with CSV export
- **AI Assistant** — Persistent chat panel with support for Gemini, OpenAI, Claude, and local LLMs

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 18, TypeScript, Vite 5, TailwindCSS 3, Zustand, Recharts |
| Desktop | Tauri 2 |
| Backend | Rust (tokio, rusqlite, reqwest, image, csv) |
| Database | SQLite (embedded, local-first) |
| AI | Provider-agnostic (Gemini API, OpenAI API, Claude API, Ollama) |

## Development

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev
```

## Build Windows Installer

```bash
npm install
npm run tauri build -- --bundles nsis,msi
```

Builds `*.msi` (WiX) and `*.exe` (NSIS) installers in `src-tauri/target/release/bundle/`.

## GitHub Actions

- **CI** — Runs TypeScript checks and Vite build on every push
- **Release** — Builds Windows MSI/EXE on tag push (`v*`) and creates a GitHub Release

## License

MIT
