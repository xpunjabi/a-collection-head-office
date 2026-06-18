# A Collection Head Office - Complete Implementation Plan

## Phase 1: Fix Blocker Issues

### 1.1 Create App.tsx
**File:** `src/App.tsx`

Main application layout component with:
- **Left Sidebar (w-56):** App logo, nav buttons (Dashboard, Catalog, Social Hub, Customers, Inventory, Automation, Reports, Settings), AI Assistant toggle at bottom
- **Center Panel (flex-1):** Renders active page based on `currentTab` from store
- **Right AI Panel (w-80, collapsible):** Chat UI with message history, input form, send button, loading state
- Uses `lucide-react` icons, dark theme consistent with existing pages
- Page mapping: dashboard→Dashboard, catalog→Catalog, social→SocialHub, customers→Customers, inventory→Inventory, automation→Automation, reports→Reports, settings→SettingsPage

### 1.2 Fix automation/mod.rs - add params import
**File:** `src-tauri/src/automation/mod.rs`

Add `params` to the rusqlite import on line 1:
```rust
// Before:
use rusqlite::Connection;
// After:
use rusqlite::{Connection, params};
```

### 1.3 Fix Reports.tsx param names
**File:** `src/pages/Reports.tsx`, line 31

Rust backend expects `start_date` / `end_date` (snake_case) but frontend sends `startDate` / `endDate` (camelCase). Tauri serializes JS camelCase keys as-is, so Rust receives wrong field names.

```ts
// Before:
const res = await invoke('get_sales_report', { startDate, endDate })
// After:
const res = await invoke('get_sales_report', { start_date: startDate, end_date: endDate })
```

### 1.4 Fix tauri.conf.json bundle targets
**File:** `src-tauri/tauri.conf.json`, line 27

Change `"targets": "all"` → `"targets": ["nsis", "msi"]` so only Windows installers are built.

### 1.5 Create .gitignore
**File:** `.gitignore`

Standard ignores:
```
node_modules/
dist/
src-tauri/target/
.DS_Store
Thumbs.db
*.log
```

## Phase 2: GitHub Actions & CI/CD

### 2.1 Create CI workflow
**File:** `.github/workflows/ci.yml`

Triggers: push to master, pull requests

Jobs:
- **lint:** `npm ci && npm run build` (TypeScript compile check via `tsc`)
- No actual test/compile since user said testing happens on GitHub Actions
- Note: `npm run build` runs `tsc && vite build` - this will verify TypeScript

### 2.2 Create Release workflow  
**File:** `.github/workflows/release.yml`

Triggers: tag push (e.g., `v*`)

Jobs:
- **build-windows:** 
  - Install dependencies
  - Run `npm run build` (frontend)
  - Run `cargo tauri build` (with `--target x86_64-pc-windows-msi` or default)
  - Upload artifacts: `*.msi` and `*.exe` (NSIS)
  - Create GitHub Release

### 2.3 Create README.md
**File:** `README.md`

Brief project description, tech stack, dev setup, build instructions for Windows.

## Phase 3: Missing Features

### 3.1 Create Settings.tsx
**File:** `src/pages/Settings.tsx`

Settings page with:
- AI Provider dropdown (Gemini, OpenAI, Claude, Local)
- API Key input (masked)
- Model name input
- Backup path selector (using Tauri dialog)
- Backup interval setting
- "Database Backup Now" button
- Theme toggle (dark only by default)
- Uses `invoke` for `get_settings` / `update_setting`

### 3.2 Fix Catalog.tsx - remove window.__TAURI__
**File:** `src/pages/Catalog.tsx`

Line 172 uses `window.__TAURI__.core.invoke(...)` which is a bad pattern (internal API). Replace with proper `import { invoke } from '@tauri-apps/api/core'`. However the file already imports `invoke` from the store. The actual fix: the CSV import function should use the already-imported Tauri dialog + fs plugin properly. The code on lines 170-186 has unnecessary inline logic - clean it up to simply read file and invoke import.

## Phase 4: Git Operations

### 4.1 Initialize & First Commit
```bash
git add .
git commit -m "feat: initial release - A Collection Head Office v0.1.0"
git remote add origin <url>
git push -u origin master
```

### 4.2 Monitor GitHub Actions
Watch CI workflow run. If it fails:
- Check workflow logs
- Fix issues locally
- Push fixes

### 4.3 Create First Release
```bash
git tag v0.1.0
git push origin v0.1.0
```
This triggers the release workflow to build Windows MSI + EXE and create a GitHub Release.

## Technical Notes

- **Build only Windows MSI/EXE** - tauri.conf.json targets set to `["nsis", "msi"]`
- **No local testing/compiling** - all CI/CD happens on GitHub Actions
- **Rust backend** uses SQLite with bundled feature (no external deps needed)
- **Frontend** uses Vite with React + TypeScript + TailwindCSS
- **AI providers:** Gemini, OpenAI, Claude, Local (Ollama) - configurable in Settings
