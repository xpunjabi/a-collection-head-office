# A Collection Head Office

A lightweight, local-first, AI-assisted business operating system for small clothing businesses.

Built with **Tauri 2**, **React**, **TypeScript**, **Rust**, and **SQLite**.

## Features

- **Dashboard** — Profit-mode overview: stock distribution, agent balances, stale stock alerts, recent shares
- **Catalog** — Product master with landed cost, profit-mode columns (HO stock, agent stock, sold qty), SOLD badges, sale recording, multi-select + social sharing
- **Share Center** — Aggressive social media: share pack generator (5 platforms), bulk WhatsApp broadcast, stale stock detector, share history log
- **Agents** — Agent management with stock + cash ledger, outstanding balance tracking, send/return/sell/cash/adjust actions
- **Purchase Trips** — Faisalabad buying trips with proportional expense allocation and landed unit cost calculation
- **Customers** — Customer profiles with segments for bulk broadcasting
- **Inventory** — Stock tracking, low stock alerts, dead stock audit, best sellers
- **Automation** — Scheduled backup and weekly report generation
- **AI Assistant** — Business-aware multi-turn chat with Gemini/OpenAI/Claude support. AI knows stock levels, agent balances, sales trends, and share history.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 18, TypeScript, Vite 5, TailwindCSS 3, Zustand |
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
- **Release Windows Build** — Builds Windows MSI/EXE on tag push (`v*`) and creates a GitHub Release

## License

MIT
