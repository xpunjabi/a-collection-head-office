# A Collection Head Office

A lightweight, local-first, AI-assisted business operating system for small clothing businesses.

Built with **Tauri 2**, **React**, **TypeScript**, **Rust**, and **SQLite**.

## Live Web Preview

> **Note:** The web preview shows the UI shell only. Backend Tauri commands (database, AI calls, file I/O) require the desktop app. For full functionality, download the Windows installer from the latest [GitHub Release](https://github.com/xpunjabi/a-collection-head-office/releases/latest).

🌐 **Web Preview:** https://a-collection-head-office.web.app/

## Features

- **Dashboard** — Business overview with stats, charts, and quick actions
- **Catalog** — Product management with CRUD, CSV import/export, image upload, multi-select + social sharing
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
| Hosting | Firebase Hosting (web preview) |

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
- **Release Windows Build** — Builds Windows MSI/EXE on tag push (`v*`) and creates a GitHub Release
- **Firebase Deploy** — Deploys the web preview to Firebase Hosting on frontend changes

## License

MIT

