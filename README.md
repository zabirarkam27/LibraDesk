# LibraDesk

LibraDesk is a Windows desktop project for managing and reading a personal PDF library offline. The desktop app can scan local folders and add PDFs to its SQLite catalog after you review the results. See the [implementation roadmap](docs/plan.md) for task status and the [detailed plan](plan.md) for requirements.

## Local development

Use Node.js 20.19+ on the 20.x line or 22.12+, npm, and the Rust MSVC toolchain. Windows builds also require the Microsoft C++ build tools and WebView2. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for setup instructions.

```powershell
npm ci
npm run tauri dev
```

Open **Settings** in the desktop window to verify the database connection and change the app and reader themes. Both preferences are stored in `%LOCALAPPDATA%\LibraDesk\database\library.sqlite3`. Run `npm run dev` for the browser preview, which uses browser storage for appearance preferences.

Open **Import**, choose **Add folders**, then select **Scan folder** beside an enabled root. The preview shows what LibraDesk found. Select **Review import** and confirm to add eligible paths to **Library**. The app reads the originals in place. An interrupted scan appears as **Paused** when the app reopens. A previously confirmed import resumes automatically.

Imported PDFs receive metadata, a SHA-256 digest, and a first-page cover in the background. Open **Metadata** to review missing titles and authors, edit books individually or in bulk, and review groups of identical files. Manual edits affect the catalog and survive rescans.

## Build checks

```powershell
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug --no-bundle
```

The last command builds a debug desktop executable. Installer packaging is tracked in the project plan.

## Project documentation

- [Windows environment audit](docs/environment-audit.md)
- [Architecture](docs/architecture.md) and [decisions](docs/decisions.md)
- [Database schema](docs/database-schema.md)
- [Development log](docs/development-log.md)
