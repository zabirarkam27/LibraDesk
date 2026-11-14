<p align="center">
  <img src="docs/assets/libradesk-banner.svg" alt="LibraDesk — Your books. Your space." width="100%" />
</p>

<p align="center">
  <strong>A personal PDF library that lives on your Windows PC.</strong><br />
  Organize thousands of books, search বাংলা and English, and read without moving your original files.
</p>

<p align="center">
  <a href="https://github.com/zabirarkam27/LibraDesk/releases/download/v0.1.1/LibraDesk_0.1.1_x64-setup.exe"><strong>Download for Windows x64 · v0.1.1</strong></a>
  &nbsp;·&nbsp;
  <a href="https://github.com/zabirarkam27/LibraDesk/releases/latest">Latest release</a>
  &nbsp;·&nbsp;
  <a href="docs/user-guide.md">User guide</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/zabirarkam27/LibraDesk/issues">Report an issue</a>
</p>

<p align="center">
  <img alt="Platform: Windows 11 x64" src="https://img.shields.io/badge/platform-Windows%2011%20x64-304c39" />
  <img alt="Offline-first" src="https://img.shields.io/badge/offline-first-b3d3af?labelColor=304c39" />
  <img alt="Tauri 2 and Rust" src="https://img.shields.io/badge/desktop-Tauri%202%20%2B%20Rust-304c39" />
  <img alt="Local SQLite storage" src="https://img.shields.io/badge/storage-local%20SQLite-304c39" />
</p>

---

## Why LibraDesk?

A large PDF collection is easy to accumulate and difficult to use. Books spread across HDDs and SSDs, filenames are inconsistent, duplicates appear, and a folder tree cannot express every subject or reading list.

LibraDesk turns that collection into a searchable personal library. It catalogs PDFs in their existing locations and stores your organization, notes, bookmarks and reading history in a local SQLite database. You can browse by category, tag, collection, favorite or reading status, then open a book in the built-in reader.

**Your PDFs stay where you put them.** Importing does not copy, move, rename or delete the originals. Core features work locally without accounts, subscriptions, cloud services or an application server.

## Contents

- [Download](#download)
- [System requirements](#system-requirements)
- [Installation](#installation)
- [Your first library](#your-first-library)
- [Features](#features)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Local data, privacy and backup](#local-data-privacy-and-backup)
- [Technology stack](#technology-stack)
- [Architecture](#architecture)
- [Performance and verification](#performance-and-verification)
- [Build from source](#build-from-source)
- [Troubleshooting and limitations](#troubleshooting-and-limitations)
- [বাংলায় দ্রুত শুরু](#বাংলায়-দ্রুত-শুরু)
- [Documentation and contributing](#documentation-and-contributing)

## Download

| Download | What it contains |
| --- | --- |
| **[LibraDesk_0.1.1_x64-setup.exe](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.1.1/LibraDesk_0.1.1_x64-setup.exe)** | Windows x64 setup with the Microsoft WebView2 Evergreen standalone installer bundled |
| [SHA256SUMS.txt](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.1.1/SHA256SUMS.txt) | SHA-256 checksum for verifying the downloaded setup |
| [Release notes](https://github.com/zabirarkam27/LibraDesk/releases/tag/v0.1.1) | Version changes, installation notes and verification scope |
| [All releases](https://github.com/zabirarkam27/LibraDesk/releases) | Published versions and assets |

Download the **setup.exe** asset. GitHub's “Source code (zip)” and “Source code (tar.gz)” archives are for developers; they are not Windows installers.

Current setup size: **224,490,298 bytes (~214.09 MiB)**. The checksum file identifies the exact release artifact.

The setup is larger than the application because it includes the full WebView2 prerequisite. Once downloaded, it can install the missing runtime locally without fetching a bootstrapper from the internet. A suitable existing runtime is reused. Microsoft's Evergreen runtime can receive its own updates when the PC is online. See the [Tauri installer documentation](https://v2.tauri.app/distribute/windows-installer/#offline-installer) and [Microsoft's runtime distribution documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution#offline-deployment).

**You do not need Node.js, npm, Rust, Visual Studio, Python or Git to install and use the app.**

## System requirements

This release provides a **Windows 11 x64 installer for Intel/AMD PCs**. There are no native ARM64, 32-bit Windows, macOS or Linux installers in this release.

| Component | Suggested minimum | Recommended for a large library |
| --- | --- | --- |
| Operating system | Windows 11, 64-bit x64 | Windows 11 with current system updates |
| Processor | Windows 11-compatible Intel/AMD x64 CPU, 2 cores | Modern 4-core or better CPU |
| Memory | 8 GB RAM | 16 GB RAM or more for large/image-heavy PDFs |
| Free space | Allow at least 2 GB for download, setup, runtime and working space | 5 GB or more, plus room for growing indexes, covers and backups |
| Source storage | Accessible local HDD or SSD containing your PDFs | SSD for app data; source PDFs may stay on HDDs |
| Display | 1280 × 720 or larger | 1920 × 1080 or larger |
| WebView2 | Bundled standalone installer supplies it if missing | Keep the runtime updated |
| Network | Needed to obtain the download; not needed for core use | Optional for downloading newer releases/runtime updates |
| Permissions | Write access to your user profile and readable source folders | A normal personal Windows account |

These are **practical planning recommendations**, not a measured lower hardware limit. The release was tested on Windows 11 Pro with a Ryzen 5 8500G and about 32 GB installed RAM. It has not been benchmarked on every 8 GB PC, Windows build or storage device. Large image PDFs and sustained indexing can require more resources. An organization-managed PC may require IT approval for application/runtime installation.

## Installation

1. Download [LibraDesk_0.1.1_x64-setup.exe](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.1.1/LibraDesk_0.1.1_x64-setup.exe) from the official repository.
2. Optional: compare the downloaded file's SHA-256 with [SHA256SUMS.txt](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.1.1/SHA256SUMS.txt):
   ~~~powershell
   Get-FileHash "$HOME\Downloads\LibraDesk_0.1.1_x64-setup.exe" -Algorithm SHA256
   ~~~
3. Close a running LibraDesk instance, then double-click the setup file.
4. Follow the setup wizard and choose the installation location. The installer targets the current user. It supplies WebView2 if it is missing.
5. Launch LibraDesk from the Start menu or its installed executable.
6. Open **Import → Add folders** and build your library.

The release is currently **unsigned**. Windows may display an unknown-publisher or SmartScreen prompt. Check the repository, release version and checksum before deciding whether to run it. You do not need to disable antivirus or Windows security.

The default application location is <code>%LOCALAPPDATA%\LibraDesk</code>. Selecting a different installation location does not relocate the local library database.

### Update or uninstall

Download a newer installer, back up your library, close LibraDesk and run setup. Saved library data remains in AppData. Silent/update-mode downgrades are refused; newer-version backups cannot be restored into an older app.

Uninstall through **Windows Settings → Apps → Installed apps → LibraDesk**, or use the installed uninstaller. Normal uninstallation removes the program and retains library data and original PDFs. Do not manually delete AppData if you want to preserve your library. See the [installation guide](docs/installation.md).

There is no automatic app updater in this version; check [Releases](https://github.com/zabirarkam27/LibraDesk/releases/latest) for newer installers.

## Your first library

1. **Add folders.** Register the HDD/SSD folders that contain your PDFs.
2. **Scan and review.** See new, existing, duplicate-candidate, invalid and inaccessible entries.
3. **Confirm import.** Add approved records; original files remain in place.
4. **Let processing continue.** Metadata, hashes, covers and searchable page text are generated in the background.
5. **Organize.** Assign categories and tags, create collections, mark favorites and set reading status.
6. **Read.** Open a book, select **Read**, and use bookmarks, notes and saved progress.
7. **Back up.** Use **Settings → Backup** to save library data independently from your source-PDF backups.

Paused discovery can resume after a restart, and a previously confirmed import resumes without duplicating paths. Search becomes more complete as background extraction progresses.

## Features

| Area | What you can do |
| --- | --- |
| Import | Register multiple folders, scan recursively, preview results, confirm import, pause/resume/cancel and inspect scan history |
| Metadata | Extract PDF fields, fall back to filenames, edit metadata individually or in bulk and preserve manual edits on rescans |
| Library | Browse virtualized Grid/List/Compact views; combine filters, sorting and text queries |
| Organization | Use nested categories, tags, favorites, reading statuses, collections and manually ordered reading queues |
| Bilingual search | Search Bengali, English and mixed metadata; find indexed PDF page content with snippets and page-aware navigation |
| Reader | Navigate pages, zoom, fit width/page, use fullscreen, contents, lazy thumbnails and in-document text search |
| Notes and progress | Keep page-linked notes and bookmarks, resume reading and save independent reader appearance/layout preferences |
| Recovery | Detect missing/inaccessible/invalid or changed files; inspect and confirm a replacement link while retaining book identity |
| Duplicates | Review exact SHA-256 groups, choose a preferred copy, ignore groups or explicitly remove a database record |
| Analytics | View manual completed/reading totals, session page views, active time, recent activity and reading history |
| Maintenance | Inspect storage usage, clear protected cache categories and create/inspect/restore validated local backups |
| Appearance | Choose Light, Dark or System app appearance and separate reader appearance |

Bengali and English support applies to library metadata, filenames and searchable text. The current interface is primarily English.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+K | Open library search and choose metadata or PDF contents |
| Ctrl+F | Search within the open PDF |
| Arrow keys / Page Up / Page Down | Navigate reader pages |
| Home / End | First / last reader page |
| + / − | Zoom in / out |
| F11 | Toggle reader fullscreen |
| Escape | Exit fullscreen |

Reader shortcuts can be disabled in preferences. Input fields retain their normal typing behavior.

## Local data, privacy and backup

App-owned data is stored under:

~~~text
%LOCALAPPDATA%\LibraDesk\
├── database\       SQLite library, settings, notes and progress
├── thumbnails\     Generated covers
├── cache\          Derived cache and restored cover generations
├── backups\        Managed library snapshots and safety backups
├── logs\           Local application diagnostics
└── temp\           Temporary processing and restore staging
~~~

- **Original PDFs:** stay at their saved source paths; LibraDesk reads them in place.
- **Local catalog:** stores metadata, organization, notes, bookmarks, sessions and extracted page text.
- **No application account or cloud sync:** core work stays on the PC. There is no telemetry integration in the application.
- **Backups:** contain library data and optional thumbnails, are unencrypted and exclude original PDFs. Keep a separate backup of your PDF folders.
- **Restore:** inspects hashes/version/schema, stages compatible data, asks for replacement acknowledgement, creates a safety snapshot and rebuilds search indexes. Reload when prompted.
- **Record removal:** does not delete a source PDF. Removing a book's last file record also removes that book's catalog notes, bookmarks, progress and memberships after explicit confirmation.

The underlying Microsoft runtime may maintain itself independently through Windows/Edge Update. Offline-first describes LibraDesk's core features, not a promise that Windows makes no network requests.

Read the [backup and recovery guide](docs/backup-recovery.md) before relying on a snapshot as your only copy.

## Technology stack

The stack below reflects the implemented code and installed dependencies.

| Layer | Technology | Role |
| --- | --- | --- |
| Desktop shell | Tauri 2 + Microsoft WebView2 | Native Windows application and embedded UI |
| Native backend | Rust, edition 2021 | Scanning, validated file access, processing, recovery and native commands |
| Interface | React 19 + TypeScript 5.9 | Component-based UI with strict types |
| Build | Vite 8 | Frontend bundling, lazy routes and local PDF assets |
| Styling | Tailwind CSS 4 | Theme tokens, layout and responsive styling |
| UI primitives | shadcn-style shared components + Radix UI | Dialogs, menus, controls and accessible interactions |
| Icons and motion | Lucide React + Framer Motion | Iconography and transitions |
| Routing and state | React Router 7 + Zustand 5 | Workspace navigation and preferences/state |
| Database | SQLite through rusqlite | Local transactions, migrations, WAL, relationships and persistence |
| Search | SQLite FTS5 + Unicode normalization | Metadata ranking and page-aware bilingual text search |
| PDF reader | PDF.js 6 | Range-based rendering, contents, thumbnails and text processing |
| PDF metadata | lopdf + bounded PDF.js fallback | Native metadata parsing and fallback processing |
| Integrity | SHA-256 + validated PNG processing | Exact duplicates, backup integrity and cached-cover validation |
| Packaging | NSIS via the Tauri CLI | Current-user Windows setup and bundled runtime |
| Quality checks | Rust tests + Node/CDP browser runners | Real backend fixtures, native smoke checks and browser regressions |

End users install the packaged application; developer tools are only needed when building the source.

## Architecture

~~~text
React UI running in WebView2
            │
     Typed Tauri commands
            │
     Rust application services
       ┌────┴───────────────┐
       │                   │
 SQLite + FTS5      Registered source PDFs
 local library      validated read-only access
       │
 AppData caches, snapshots and processing checkpoints
~~~

The frontend never receives unrestricted filesystem or SQL access. Native commands validate operations, long-running work is bounded and resumable, and derived data is associated with source revisions. Backup/restore coordinates workers and creates a safety copy before replacement.

See [architecture](docs/architecture.md), [database schema](docs/database-schema.md) and [security review](docs/security-review.md).

## Performance and verification

Recorded collection tests used 100, 1,000, 5,000 and 10,001 generated PDFs. On the documented development machine, the 10,001-record native idle-catalog run measured:

| Metric | Recorded result | Agreed budget |
| --- | ---: | ---: |
| Launch to visible overview | ~705 ms | ≤3,000 ms |
| Warm metadata search including IPC, maximum of 10 samples | ~91.91 ms | ≤500 ms |
| App + descendant WebView2 working set, maximum of 5 samples | ~457.91 MiB | ≤500 MiB |

These are measured QA results, not universal hardware guarantees. The native scale run used an embedded-frontend debug build and idle catalog with parsing queues removed. Heavy indexing and complex PDF rendering have different resource needs.

Verification also covers 46 ordinary Rust tests, ten browser suites with mock native IPC and real PDF.js, cross-feature real SQLite/file integration, a 2,000-page PDF, a source over 300 MiB, restore rollback, SQL/FTS input handling and Windows junction safety. Installer checks verify versions, expected binary bytes, startup/offline navigation, uninstall and PDF/data preservation.

The bundled-runtime installer is checked on the current Windows machine; a fresh OS with no runtime is not claimed as a tested environment. See [performance details](docs/performance-report.md), [test strategy](docs/testing-strategy.md) and the [release report](docs/release-report.md).

## Build from source

### Developer prerequisites

- Windows 11 x64 for the tested desktop target.
- Node.js 20.19+ on the 20.x line, or 22.12+; npm.
- Rust MSVC toolchain, Visual Studio C++ build tools and Windows SDK.
- WebView2 for development. See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
- Network access for uncached dependencies and the initial standalone-runtime packaging download.

~~~powershell
git clone https://github.com/zabirarkam27/LibraDesk.git
cd LibraDesk
npm ci
npm run tauri dev
~~~

Frontend-only preview:

~~~powershell
npm run dev
~~~

The preview does not have the desktop app's local library/filesystem access.

### Production installer

~~~powershell
npm run tauri build -- --bundles nsis -- --locked
~~~

Output: <code>src-tauri/target/release/bundle/nsis/LibraDesk_0.1.1_x64-setup.exe</code>.

The runtime is included through Tauri's offlineInstaller packaging mode. First packaging may download Microsoft's standalone installer; cached Cargo dependencies do not eliminate that prerequisite download.

### Checks

~~~powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run test:backend
~~~

For browser regressions, install Python's reportlab and Pillow packages and use Chrome at its standard Windows path:

~~~powershell
python scripts/make-reader-fixtures.py
npm run test:browser
~~~

Native/installer QA and performance commands are documented in the [testing strategy](docs/testing-strategy.md). Installer QA uses a separate destination and refuses an existing registered installation.

## Troubleshooting and limitations

| Situation | What to check |
| --- | --- |
| Windows says the publisher is unknown | This release is unsigned. Verify the official download and checksum; do not disable security software. |
| Setup/runtime installation is blocked | Check free space, user permissions, organizational policy and whether Windows needs a restart. |
| App fails after an old runtime installation | Update WebView2 using [Microsoft's standalone runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) and retry. |
| A disk or source PDF is missing | Reconnect the drive, run verification or use Relink PDF; library records remain available. |
| Search finds only some pages | Check extraction coverage and current file revision. Indexing pauses while the reader is open or the window is hidden. |
| A scanned page has no searchable words | OCR is outside this release; embedded PDF text is required. |
| An encrypted book is not indexed | Open it manually with its password; background extraction cannot unlock it. |
| A restored book path is unavailable | Restore preserves saved paths. Relink to the PDF's current location. |

There is no OCR, account system, cloud synchronization, scheduled backup or automatic application updater in v0.1.1. Search/indexing, reader resource bounds and backup privacy limitations are described in [known limitations](docs/known-limitations.md).

## বাংলায় দ্রুত শুরু

1. উপরের **Download for Windows x64** লিংক থেকে setup.exe ডাউনলোড করুন।
2. Windows 11-এর Intel/AMD 64-bit পিসিতে installer চালান। Node.js, Rust বা অন্য developer tool লাগবে না।
3. WebView2 না থাকলে setup-এর সঙ্গে থাকা standalone installer সেটি ইনস্টল করবে; setup ডাউনলোড হয়ে গেলে এই prerequisite-এর জন্য আলাদা internet download লাগে না।
4. অ্যাপের **Import → Add folders** থেকে PDF-এর folder বেছে scan করুন।
5. Preview দেখে import confirm করুন। মূল PDF সরানো, rename বা delete করা হবে না।
6. Library থেকে বই খুলে পড়ুন, category/tag দিন এবং notes/bookmarks ব্যবহার করুন।
7. **Settings → Backup** থেকে library backup রাখুন। মূল PDF-গুলোর backup আলাদাভাবে সংরক্ষণ করুন।

Interface মূলত English; বাংলা filename, metadata ও searchable PDF text সমর্থিত। বিস্তারিত জানতে [user guide](docs/user-guide.md) দেখুন।

## Documentation and contributing

| Document | Purpose |
| --- | --- |
| [User guide](docs/user-guide.md) | Daily library and reader workflows |
| [Installation](docs/installation.md) | Setup, updates, runtime and uninstall |
| [Backup and recovery](docs/backup-recovery.md) | Snapshot format, restore and safety recovery |
| [Troubleshooting](docs/troubleshooting.md) | Common problems and safe next steps |
| [Known limitations](docs/known-limitations.md) | Supported scope and resource limits |
| [Architecture](docs/architecture.md) · [Database](docs/database-schema.md) | Technical design and persistence |
| [Testing](docs/testing-strategy.md) · [Performance](docs/performance-report.md) · [Security](docs/security-review.md) | Evidence, methodology and review scope |
| [Release report](docs/release-report.md) · [Development log](docs/development-log.md) | Delivered artifacts and implementation history |
| [Roadmap](docs/plan.md) · [Detailed plan](plan.md) | Completed phases and acceptance checklist |

For a bug report, include your Windows/app version, steps, expected/actual behavior and safe error text. Use [GitHub Issues](https://github.com/zabirarkam27/LibraDesk/issues). Do not upload personal PDFs, library databases, notes, backups or private file paths.

When contributing code, preserve original source files, keep native operations validated, use transactions/migrations and run the relevant checks. Generated PDFs, databases, caches, local profiles and installers belong outside version control. Third-party components retain their respective licenses.
