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

Open a book in **Library** and select **Read**. The reader supports page navigation, fit and custom zoom, fullscreen, contents, lazy thumbnails, and searching embedded PDF text. Use the right sidebar for bookmarks, page-linked notes, and reader settings. Page position resumes after reopening. Reader appearance and layout are saved separately from app appearance.

Use the arrow keys or Page Up/Down to change pages, Home/End for the first/last page, +/− to zoom, Ctrl+F to find text, and F11 for fullscreen. Escape exits fullscreen. Reading status can be changed in the book editor.

Open **PDF contents** to search extracted pages across your library in Bengali or English. Ctrl+K also offers **PDF contents** in the search scope. Each result shows the book, source file, page, and a short excerpt; select its title to open that page. Search matches word prefixes, and all entered words must match on the same page.

Background extraction runs while the window is visible and the reader is closed. **Book details → Files** shows progress and manual retry for failed files. Search includes pages saved so far; select **Refresh results** as extraction progresses. Excerpts use normalized text, while original extracted text is retained separately. Image-only PDFs have no embedded text; OCR is excluded. Background extraction cannot unlock encrypted PDFs.

Open **File recovery** to see missing, inaccessible, and invalid PDFs. File checks run in the background at startup, resume after interruption, and repeat hourly. **Check files now** starts another pass when the current pass has finished. Clear **Show only unavailable files** to inspect available files and change results.

Select **Relink PDF** in File recovery or Book details, choose a replacement, compare its SHA-256 and metadata, then confirm. Cancelling preserves the current link. Relinking retains the book, notes, bookmarks, and reading progress. A different replacement advances the file revision, so old page anchors may need review. Original and replacement PDFs remain in place.

In **Metadata → Exact duplicates**, compare copies, save a preferred record, or ignore a group while retaining every copy. **Remove library record** requires a separate confirmation and acknowledgement. The kept copy is rehashed before removal. Removing a book's last file also removes that book's library notes, bookmarks, progress, and collection memberships; its PDF stays on disk. A book with other files retains its book-level records.

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
