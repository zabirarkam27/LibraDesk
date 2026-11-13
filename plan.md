# LibraDesk - Detailed Development Plan

**Project:** LibraDesk
**Application Type:** Windows Desktop Application
**Architecture:** Offline-first, Local-first
**Primary Purpose:** Personal PDF Library Management
**Initial Collection:** Approximately 30 GB
**Target Scale:** 10,000+ PDF records
**Development Approach:** Task-by-task, phase-based implementation with mandatory user approval.

---

# 1. PROJECT OBJECTIVE

Build a professional, reliable, modern Windows desktop application named LibraDesk for managing, organizing, searching, and reading personal PDF books stored on local HDDs and SSDs.

The application must:

- Work completely offline for all core features.
- Preserve original PDF files and their locations.
- Organize books virtually through SQLite metadata.
- Support Bengali and English.
- Provide an integrated PDF reader.
- Extract PDF metadata automatically.
- Generate book cover thumbnails.
- Detect duplicate and missing files.
- Support thousands of PDF records efficiently.
- Store application data inside Windows AppData.
- Provide reliable backup and recovery.

## 1.1 Final Technology Stack

| Layer             | Technology     |
| ----------------- | -------------- |
| Desktop Framework | Tauri 2        |
| Frontend          | React          |
| Language          | TypeScript     |
| Build Tool        | Vite           |
| Styling           | Tailwind CSS   |
| UI Components     | shadcn/ui      |
| Icons             | Lucide React   |
| Animation         | Framer Motion  |
| Database          | SQLite         |
| Search            | SQLite FTS5    |
| Native Backend    | Rust           |
| PDF Rendering     | PDF.js         |
| State Management  | Zustand        |
| Validation        | Zod            |
| Data Tables       | TanStack Table |

Do not introduce unnecessary technologies or backend servers.

## 1.2 Product Scope

Included:

- PDF book management.
- Virtual categories.
- Nested categories.
- Tags.
- Collections.
- Favorites.
- Reading lists.
- Metadata editing.
- PDF scanning and importing.
- Duplicate detection.
- Missing file recovery.
- Built-in PDF reader.
- Reading progress.
- Bookmarks.
- Personal notes.
- Bilingual search.
- Local database.
- Backup and restore.

Excluded:

- AI features.
- Cloud synchronization.
- Online accounts.
- Physical book management.
- OCR in the initial versions.
- Automatic file moving or renaming.
- Automatic deletion of source files.

---

# 2. MOST IMPORTANT DEVELOPMENT RULE: USER APPROVAL GATE

This is the most important instruction in the entire project.

**Codex must complete ONE TASK at a time. After every individual task, it must stop and ask the user for explicit confirmation before proceeding.**

This rule applies to every task, including small tasks within the same phase.

## 2.1 Required Task Workflow

Follow this exact workflow:

1. Read the task requirements.
2. Inspect relevant existing code and dependencies.
3. Identify the files that need modification.
4. Explain the implementation approach.
5. Implement only the current task.
6. Run relevant tests and checks.
7. Review all changes.
8. Update project documentation.
9. Report the completed task.
10. Stop and wait for user confirmation.

Never automatically continue to the next task.

## 2.2 Mandatory Confirmation Protocol

After completing each task, present this report:

```text
========================================
LIBRADESK TASK COMPLETION REPORT
========================================

Phase:
Task Number:
Task Name:

Status: COMPLETED / PARTIALLY COMPLETED / BLOCKED

Implementation Summary:
- ...

Files Created:
- ...

Files Modified:
- ...

Database Changes:
- ...

Tests Executed:
- ...

Test Results:
- ...

Build Status:
- ...

Known Issues:
- ...

Documentation Updated:
- ...

Git Commit:
- ...

Git Push:
- ...

========================================
AWAITING USER APPROVAL
========================================

The current task has been completed.

Would you like me to proceed with the next task?

1. Approve and continue.
2. Request changes.
3. Investigate reported issues.
4. Stop development.

Do not begin another task until the user responds.
========================================
```

## 2.3 Approval Rules

- A task is not approved merely because the previous task was completed.
- A phase is not approved merely because its first task was approved.
- Explicit user approval is required after every task.
- If the user requests changes, modify only the relevant current task.
- If tests fail, report the failure honestly.
- Never mark incomplete functionality as completed.
- Never skip a task without explicit permission.
- Never interpret silence as approval.
- Never automatically start the next task after reporting completion.
- If the user approves with additional instructions, follow those instructions before proceeding.

**Important:** Even if multiple tasks are technically related, complete them separately and obtain confirmation after each one.

---

# 3. GLOBAL DEVELOPMENT RULES

These rules apply to every phase.

## 3.1 Inspect Before Changing

Before editing anything:

- Inspect repository structure.
- Read package.json.
- Inspect installed dependencies.
- Inspect Tauri configuration.
- Inspect Rust modules.
- Inspect database configuration.
- Inspect existing routes and components.
- Inspect styling and shared components.
- Inspect existing tests.
- Inspect build configuration.

Never assume existing functionality.

## 3.2 Code Quality

- Use strict TypeScript.
- Use modular, feature-based architecture.
- Keep components small and maintainable.
- Separate UI, business logic, database and native operations.
- Use reusable hooks and services.
- Avoid duplicated logic.
- Avoid unnecessary dependencies.
- Handle errors explicitly.
- Avoid unsafe Rust where unnecessary.
- Maintain readable naming conventions.

## 3.3 Data Safety

- Never automatically move original PDF files.
- Never automatically rename original PDF files.
- Never automatically delete original PDF files.
- Never overwrite source documents.
- Preserve original paths.
- Preserve user-edited metadata.
- Use database transactions.
- Implement database migrations.
- Validate file paths before native operations.

## 3.4 Performance

- Use background scanning.
- Use bounded concurrency.
- Avoid loading all PDFs into memory.
- Use thumbnail caching.
- Use database indexes.
- Use virtualized book grids.
- Avoid blocking the UI thread.
- Implement resumable processing.
- Avoid unnecessary file hashing.
- Use incremental processing when possible.

## 3.5 Git Rules

At the end of every approved task:

- Review changed files.
- Run relevant tests.
- Run the appropriate build check.
- Update documentation.
- Create a meaningful commit when the task is ready.
- Push only to the configured remote repository.

Never commit:

- Personal PDF files.
- Local SQLite databases.
- Private backups.
- Secrets.
- Temporary files.
- Generated personal thumbnails.

Do not claim a successful Git push unless the push actually succeeds.

---

# PHASE 0: REPOSITORY INSPECTION AND PLANNING

**Objective:** Understand the environment and prepare the complete implementation architecture without prematurely changing application code.

## Task 0.1 - Repository Inspection

- [x] Completed on 2026-11-10.
- Inspection found only the root plan. Application source, configuration, dependencies, tests, and README are absent.
- Verified by filesystem inventory and Git checks. Git reported that the workspace is outside a repository. Application tests and builds were unavailable.
- Known issue. Git tracking and remote configuration could not be inspected. The plan also places Tauri initialization before a frontend scaffold exists and lists personal notes in the release criteria without assigning them an implementation task.
- Commit reference. Unavailable because the workspace has no Git repository.

Inspect:

- Root directory.
- Existing source code.
- package.json.
- TypeScript configuration.
- Vite configuration.
- Tailwind configuration.
- Tauri configuration.
- Rust configuration.
- Existing dependencies.
- Existing tests.
- Git status.
- Existing README.

Deliver:

- Repository structure report.
- Existing implementation summary.
- Missing configuration report.
- Potential technical conflicts.

Do not modify source code.

**Approval required before Task 0.2.**

### Scaffold follow-up

The initial Tauri 2 and React/Vite scaffold was added on 2026-11-10 at the project owner's request. See [README.md](README.md) for setup and verification commands. The Task 0.1 entry records the workspace state at the time of inspection.

## Task 0.2 - Development Environment Audit

- [x] Completed on 2026-11-10. Recorded Windows, Node.js, npm, Rust, Cargo, WebView2, MSVC and SDK versions in the [environment audit](docs/environment-audit.md).
- Frontend and locked Rust checks passed. The audit records network and sandbox cache warnings. Commit evidence is in the [development log](docs/development-log.md).

Verify:

- Windows version.
- Node.js version.
- npm version.
- Rust toolchain.
- Cargo.
- Tauri prerequisites.
- WebView2 availability.
- Required Windows build tools.

Report missing or incompatible dependencies.

Do not install anything without approval.

**Approval required before Task 0.3.**

## Task 0.3 - Architecture Documentation

- [x] Completed on 2026-11-10. Published the [architecture](docs/architecture.md) and [decisions](docs/decisions.md) and reviewed them against the plan and scaffold.
- The PDF range transport still needs a Windows prototype. See the [development log](docs/development-log.md).

Create:

- docs/architecture.md
- docs/decisions.md

Document:

- Frontend architecture.
- Rust backend architecture.
- Native command communication.
- SQLite architecture.
- AppData storage.
- PDF processing.
- Search architecture.
- Error handling.
- Security boundaries.

**Approval required before Task 0.4.**

## Task 0.4 - Database Documentation

- [x] Completed on 2026-11-10. Published the [database schema](docs/database-schema.md), including constraints, FTS5, migrations and recovery.
- A local SQLite prototype passed Bengali FTS5, foreign key cascade and backup checks. App-specific verification is scheduled in Task 1.7. See the [development log](docs/development-log.md).

Create:

- docs/database-schema.md

Document:

- All required database tables.
- Relationships.
- Primary keys.
- Foreign keys.
- Indexes.
- Constraints.
- FTS5 strategy.
- Migration strategy.
- Data recovery strategy.

**Approval required before Task 0.5.**

## Task 0.5 - Implementation Roadmap

- [x] Completed on 2026-11-10. Published the [roadmap](docs/plan.md) and [development log](docs/development-log.md).
- Verified the task inventory, dependency order, document links, and Phase 0 statuses. Tasks in later phases are pending approval.

Create:

- docs/plan.md
- docs/development-log.md

This file is the primary development checklist.

Document all phases and tasks, task dependencies, completion criteria and testing requirements.

Mark the current task status accurately.

**Approval required before Phase 1.**

---

# PHASE 1: DESKTOP FOUNDATION

**Objective:** Establish the Windows desktop application, core frontend architecture, database and design system.

## Task 1.1 - Initialize Tauri 2 Application

- [x] Completed on 2026-11-10. Verified Windows startup, window bounds, and native IPC in the [Phase 1 log](docs/development-log.md).

- Configure Tauri 2.
- Verify React and Vite integration.
- Verify Rust compilation.
- Configure application identifier.
- Configure application window.
- Configure Windows development build.

Acceptance criteria:

- Application launches successfully.
- No critical runtime errors.
- Native Tauri communication works.

**Approval required before Task 1.2.**

## Task 1.2 - Frontend Architecture

- [x] Completed on 2026-11-10. Added a strict TypeScript entry point, hash routes, shared providers, and feature modules.

Create the approved feature-based structure.

Set up:

- App entry point.
- Router.
- Global providers.
- Shared components.
- Feature directories.
- Hooks.
- Stores.
- Types.
- Utilities.

Acceptance criteria:

- Clean imports.
- Strict TypeScript.
- No unnecessary abstraction.
- Application builds successfully.

**Approval required before Task 1.3.**

## Task 1.3 - UI Design System

- [x] Completed on 2026-11-10. Added Tailwind theme tokens, shadcn/ui controls, Lucide icons, toast feedback, and reduced-motion support.

Implement:

- Tailwind CSS.
- shadcn/ui.
- Lucide React.
- Design tokens.
- Typography.
- Spacing.
- Border styles.
- Reusable buttons.
- Inputs.
- Dialogs.
- Dropdowns.
- Tooltips.
- Toast notifications.
- Skeleton loaders.

Acceptance criteria:

- Consistent visual language.
- Reusable components.
- Responsive desktop layout.

**Approval required before Task 1.4.**

## Task 1.4 - Application Layout

- [x] Completed on 2026-11-10. Verified four workspace routes, the application menu, search placeholder, and compact sidebar.

Implement:

- Main application shell.
- Sidebar.
- Header.
- Navigation.
- Main content area.
- Global search placeholder.
- Application menu.
- Window layout behavior.

Acceptance criteria:

- Navigation works.
- Layout remains stable.
- No broken routes.

**Approval required before Task 1.5.**

## Task 1.5 - Theme System

- [x] Completed on 2026-11-10. Verified Light, Dark, and System modes and independent reader appearance across a restart.

Implement:

- Dark theme.
- Light theme.
- System theme.
- Persistent theme preference.
- Reader theme isolation.

Acceptance criteria:

- Theme changes immediately.
- Preference persists after restart.
- All shared components support both themes.

**Approval required before Task 1.6.**

## Task 1.6 - AppData Storage Initialization

- [x] Completed on 2026-11-10. Verified automatic creation of six local directories and tested unsuitable storage paths and temporary-file cleanup.

Create the local application storage directories:

```text
%LOCALAPPDATA%\LibraDesk\
├── database/
├── thumbnails/
├── cache/
├── backups/
├── logs/
└── temp/
```

Implement:

- Automatic directory initialization.
- Path resolution.
- Permission handling.
- Safe temporary storage.
- Storage validation.

Acceptance criteria:

- Correct Windows AppData path.
- No manual directory creation required.
- Graceful handling of inaccessible storage.

**Approval required before Task 1.7.**

## Task 1.7 - SQLite Initialization

- [x] Completed on 2026-11-10. Enabled bundled SQLite with WAL and foreign keys. Verified migration rollback, database integrity, and Bengali FTS5.

Implement:

- SQLite connection.
- WAL configuration where appropriate.
- Foreign key enforcement.
- Migration system.
- Database initialization.
- Connection error handling.
- Database version tracking.

Acceptance criteria:

- Database initializes correctly.
- Migrations work.
- Application restarts without data loss.
- Database integrity checks pass.

**Approval required before Phase 2.**

---

# PHASE 2: PDF SCANNER AND IMPORTER

**Objective:** Discover PDF files from HDD folders and import their metadata safely.

## Task 2.1 - Native Folder Picker

- [x] Completed on 2026-11-11. Registered two folders from one Windows dialog and verified that cancelling the picker leaves the list empty. Tested disabling and retiring roots without altering catalog records.

Implement:

- Native Windows folder picker.
- Multiple root folder selection.
- Root folder registration.
- Root folder listing.
- Remove root from scan list.
- Enable/disable scan roots.

Important: Removing a root must never delete its books or files.

**Approval required before Task 2.2.**

## Task 2.2 - Recursive PDF Discovery

- [x] Completed on 2026-11-11. Verified nested Bengali paths, case-insensitive PDF extensions, and exclusion of a Windows junction.

Implement:

- Recursive directory traversal.
- PDF extension detection.
- Unicode path support.
- Bengali filename support.
- Permission error handling.
- Symlink and reparse-point safety.
- Directory traversal protection.

Acceptance criteria:

- Nested folders are discovered.
- Invalid paths do not crash scanning.
- Original files remain untouched.

**Approval required before Task 2.3.**

## Task 2.3 - PDF Validation

- [x] Completed on 2026-11-11. Verified read-only PDF header and metadata checks, invalid-file classification, and inaccessible paths.

Implement:

- File existence checks.
- PDF signature validation.
- File readability checks.
- File size detection.
- Last modified time.
- Invalid PDF classification.

Do not assume every file ending in `.pdf` is a valid PDF.

**Approval required before Task 2.4.**

## Task 2.4 - Scan Job System

- [x] Completed on 2026-11-11. Added background processing, progress updates, persisted checkpoints, pause, resume, and cancellation. Tested restart recovery and duplicate-job prevention.

Implement:

- Scan job creation.
- Persistent scan status.
- Background execution.
- Progress events.
- Pause.
- Resume.
- Cancel.
- Failure reporting.

Acceptance criteria:

- Scanning does not freeze the UI.
- Job state survives application restart.
- Duplicate scan jobs are prevented.

**Approval required before Task 2.5.**

## Task 2.5 - Import Preview

- [x] Completed on 2026-11-11. Verified preview counts and no catalog writes before confirmation. Equal file size flags a possible duplicate.

Display:

- Total PDFs found.
- Total storage size.
- New files.
- Existing files.
- Duplicate candidates.
- Invalid files.
- Inaccessible files.

Require user confirmation before importing new records.

**Approval required before Task 2.6.**

## Task 2.6 - Database Import

- [x] Completed on 2026-11-11. Verified transactional batches, idempotent path registration, source revalidation, rollback, and import recovery.

Implement:

- Transactional import.
- Existing file detection.
- Path-based identity.
- Import status.
- Failure recovery.
- Safe batch insertion.

Acceptance criteria:

- Repeated scanning does not create duplicate records.
- Partial failures do not corrupt the database.
- Original PDFs are never modified.

**Approval required before Task 2.7.**

## Task 2.7 - Scan History

- [x] Completed on 2026-11-11. Verified persisted results and timestamps. See the [Phase 2 log](docs/development-log.md) for the Windows smoke test and automated checks.

Implement:

- Scan history.
- Start time.
- End time.
- Root folder.
- Files discovered.
- Files imported.
- Files failed.
- Status.

**Approval required before Phase 3.**

---

# PHASE 3: METADATA, HASHING AND THUMBNAILS

**Objective:** Turn imported PDF files into properly indexed book records.

## Task 3.1 - PDF Metadata Extraction

- [x] Completed on 2026-11-11. Extracted document info and page count with lopdf and verified the PDF.js fallback on a 34 MiB Windows fixture.

Extract:

- Title.
- Author.
- Subject.
- Creator.
- Producer.
- Creation date.
- Page count.

Use reliable PDF parsing.

Preserve original metadata values.

Do not invent missing information.

**Approval required before Task 3.2.**

## Task 3.2 - Filename Fallback

- [x] Completed on 2026-11-11. Verified separator cleanup, Unicode preservation, and a fallback for empty stems.

Implement:

- Extension removal.
- Underscore cleanup.
- Hyphen cleanup.
- Whitespace normalization.
- Unicode preservation.
- Empty title handling.

If metadata is missing, use the filename as a provisional title.

Do not automatically guess author or publisher.

**Approval required before Task 3.3.**

## Task 3.3 - SHA-256 Hashing

- [x] Completed on 2026-11-11. Verified streamed hashes against independent values and rehashing only the changed fixture after rescan.

Implement:

- Streaming file hashing.
- Background processing.
- Hash persistence.
- Hash validation.
- Incremental rehashing.

Do not load entire large PDF files into memory.

**Approval required before Task 3.4.**

## Task 3.4 - Duplicate Detection

- [x] Completed on 2026-11-11. Verified exact digest grouping and saved review decisions while preserving both copies.

Implement:

- Exact hash matching.
- Duplicate groups.
- Duplicate status.
- Duplicate review records.

Do not automatically delete, merge or overwrite duplicate files.

**Approval required before Task 3.5.**

## Task 3.5 - Cover Thumbnail Generation

- [x] Completed on 2026-11-11. Verified local first-page PNGs, Windows cache reads, revision invalidation, and a malformed-PDF placeholder.

Implement:

- First-page rendering.
- Thumbnail dimensions.
- Image conversion.
- Cache storage.
- Cache invalidation.
- Error fallback.

Use the PDF source version or file identity to avoid unnecessary regeneration.

**Approval required before Task 3.6.**

## Task 3.6 - Metadata Review Queue

- [x] Completed on 2026-11-11. Verified editor cancellation, bulk save, and persistence across restart and rescan. See the [Phase 3 log](docs/development-log.md).

Implement:

- Missing metadata detection.
- Metadata editor.
- Bulk editing.
- Save and cancel.
- Manual correction persistence.

A new scan must not overwrite manually corrected metadata without explicit approval.

**Approval required before Phase 4.**

---

# PHASE 4: LIBRARY MANAGEMENT

**Objective:** Create the complete virtual library experience.

## Task 4.1 - Book Repository and Services

Completed on 2026-11-11. See the [development log](docs/development-log.md) for verification.

- [x] Book CRUD.
- [x] File repository.
- [x] Author repository.
- [x] Metadata persistence.
- [x] Database transaction handling.
- [x] Typed frontend services.

Do not put raw SQL inside UI components.

**Approval required before Task 4.2.**

## Task 4.2 - All Books Page

Completed on 2026-11-11. See the [development log](docs/development-log.md) for verification.

- [x] Grid, list, and compact views.
- [x] Book cards with covers and metadata.
- [x] Loading, empty, and retryable error states.
- [x] Row virtualization and cursor pagination for large collections.

**Approval required before Task 4.3.**

## Task 4.3 - Book Details Page

- [x] Completed on 2026-11-11. Implementation and verification are recorded in [the development log](docs/development-log.md).

Implemented:

- Cover.
- Title.
- Authors.
- Publisher.
- ISBN.
- Publication year.
- Language.
- Page count.
- File size.
- Original location.
- Metadata editing.
- Reading status.

**Approval required before Task 4.4.**

## Task 4.4 - Category Management

- [x] Completed on 2026-11-11. Nested categories, assignments, filtering, and deletion checks are verified.

Implemented:

- Create category.
- Edit category.
- Delete category.
- Nested categories.
- Category assignment.
- Multiple categories per book.
- Category filtering.

Deleting a category must not delete its books.

**Approval required before Task 4.5.**

## Task 4.5 - Tag Management

- [x] Completed on 2026-11-11. Tag CRUD, assignments, filtering, and search synchronization are verified.

Implemented:

- Create tags.
- Rename tags.
- Delete tags.
- Multiple tags per book.
- Tag filtering.

**Approval required before Task 4.6.**

## Task 4.6 - Favorites and Reading Status

- [x] Completed on 2026-11-11. Favorites and reading statuses persist in SQLite and reject stale writes.

Implemented:

- Favorites.
- Unread.
- Reading.
- Completed.
- On Hold.

All status changes must persist.

**Approval required before Task 4.7.**

## Task 4.7 - Collections

- [x] Completed on 2026-11-11. Collections, reading lists, membership, descriptions, and ordering are verified.

Implemented:

- Custom collections.
- Collection descriptions.
- Add/remove books.
- Collection sorting.
- Collection details.
- User-defined reading lists backed by collections.

**Approval required before Task 4.8.**

## Task 4.8 - Sorting and Filtering

- [x] Completed on 2026-11-11. Combined filters, stable sorting, and bounded pages are verified. Reader-generated progress will be rechecked in Task 5.6.

Implemented:

- Title.
- Author.
- Date added.
- File size.
- Publication year.
- Reading progress.
- Category.
- Tag.
- Language.
- Reading status.

**Approval required before Task 4.9.**

## Task 4.9 - Bilingual Metadata Search

- [x] Completed on 2026-11-11. FTS5, NFC normalization, ranking, literal partial matching, and Bengali fixtures are verified.

Implemented:

- English search.
- Bengali search.
- Mixed-language search.
- Unicode normalization.
- Case-insensitive matching.
- Partial search fallback.
- FTS5 integration.
- Search ranking.
- Search filters.

Test:

- বাংলা বইয়ের নাম।
- English book titles.
- Bengali author names.
- Mixed Bengali-English queries.
- Bengali vowel signs.
- যুক্তাক্ষর.
- Bengali punctuation.
- Unicode special characters.

Never modify original titles merely to improve search.

**Approval required before Task 4.10.**

## Task 4.10 - Dashboard

- [x] Completed on 2026-11-11. SQLite totals and lists are verified. Recently read uses saved progress, which the reader will populate in Task 5.6.

Implemented real database-driven statistics:

- Total books.
- Total storage.
- Total pages.
- Recently added.
- Recently read.
- Favorites.
- Currently reading.
- Category distribution.

No mock statistics in production.

**Approval required before Phase 5.**

---

# PHASE 5: BUILT-IN PDF READER

**Objective:** Provide a comfortable, reliable reading experience inside LibraDesk.

## Task 5.1 - PDF.js Integration

- [x] Completed on 2026-11-11. Bundled PDF.js, its worker, fonts, CMaps, and decoders. Verified read-only range IPC in the production-protocol Windows build.

Implemented:

- PDF.js installation.
- Worker configuration.
- Local PDF loading.
- Document lifecycle management.
- Error handling.

Verify compatibility with Tauri's production build.

**Approval required before Task 5.2.**

## Task 5.2 - Reader Layout

- [x] Completed on 2026-11-11. Added the reader toolbar, page canvas/text layer, sidebars, and responsive desktop layout.

Implemented:

- Reader toolbar.
- Main page canvas.
- Left sidebar.
- Right sidebar.
- Page navigation.
- Responsive desktop layout.

**Approval required before Task 5.3.**

## Task 5.3 - PDF Navigation

- [x] Completed on 2026-11-11. Verified bounded page navigation, direct entry, zoom, fit modes, fullscreen, and keyboard controls.

Implemented:

- Previous page.
- Next page.
- Direct page number.
- First page.
- Last page.
- Zoom in/out.
- Fit width.
- Fit page.
- Fullscreen.

**Approval required before Task 5.4.**

## Task 5.4 - Table of Contents and Thumbnails

- [x] Completed on 2026-11-11. Added lazy nested outline navigation and virtualized thumbnail rendering.

Implemented:

- PDF outline extraction.
- Table of contents navigation.
- Page thumbnails.
- Lazy thumbnail generation.
- Sidebar navigation.

**Approval required before Task 5.5.**

## Task 5.5 - PDF Text Search

- [x] Completed on 2026-11-11. Verified English and Bengali text search, next/previous matches, counts, and highlights.

Implemented:

- Search current PDF.
- Next match.
- Previous match.
- Match highlighting.
- Search result navigation.

**Approval required before Task 5.6.**

## Task 5.6 - Reading Progress

- [x] Completed on 2026-11-11. Added debounced progress, auto resume, close-time persistence, session checkpoints, and dashboard integration.

Implemented:

- Current page.
- Total pages.
- Progress percentage.
- Last read time.
- Auto resume.
- Debounced persistence.
- Reader close persistence.
- Persist reading sessions for later analytics.

Do not mark a book completed simply because its last page was opened.

**Approval required before Task 5.7.**

## Task 5.7 - Bookmarks and Personal Notes

- [x] Completed on 2026-11-11. Added bookmark and page-linked note CRUD, navigation, stale-anchor warnings, and persistence checks.

Implemented:

- Add bookmark.
- Edit bookmark label.
- Remove bookmark.
- Navigate to bookmarked page.
- Add, edit, and remove page-linked personal notes.
- Navigate from a note to its page and verify notes persist after restart.

**Approval required before Task 5.8.**

## Task 5.8 - Reader Preferences

- [x] Completed on 2026-11-11. Persisted reader appearance, zoom, layout, fullscreen, and shortcut preferences independently of app appearance.

Implemented:

- Reader dark mode.
- Reader light mode.
- Default zoom.
- Page layout.
- Fullscreen.
- Keyboard shortcuts.

Reader preferences must be persisted separately from application theme.

**Approval required before Task 5.9.**

## Task 5.9 - Reader Performance

- [x] Completed on 2026-11-11. Verified 2,000 pages, a source larger than 300 MiB, rapid navigation, merged image ranges, book switching, worker cleanup, and Windows shutdown. See the [development log](docs/development-log.md).

Tested:

- Large PDFs.
- Long documents.
- Rapid page navigation.
- Switching between books.
- Memory cleanup.
- Worker lifecycle.
- Application shutdown.

**Approval required before Phase 6.**

---

# PHASE 6: ADVANCED SEARCH AND FILE RECOVERY

**Objective:** Make LibraDesk capable of handling large, changing PDF collections reliably.

## Task 6.1 - PDF Text Extraction

- [x] Completed on 2026-11-11. Added bounded background page extraction, transactional checkpoints, lease-based resume, per-file status, and manual failure retry. Verified Bengali/English PDFs, restart recovery, revision invalidation, and the native pipeline. See the [development log](docs/development-log.md).

Implement:

- Background page text extraction.
- Per-page text records.
- Processing status.
- Failure handling.
- Resumable indexing.

**Approval required before Task 6.2.**

## Task 6.2 - Full-text Search Index

- [x] Completed on 2026-11-13. Added migration 0008 with the page FTS5 index, transactional synchronization and backfill, bilingual ranked page search with bounded snippets, and revision-aware navigation from results into the reader. See the [development log](docs/development-log.md).

Implement:

- Dedicated FTS5 content index.
- Page-aware results.
- Search result snippets.
- Book identification.
- Result navigation.
- Index maintenance.

OCR is excluded.

**Approval required before Task 6.3.**

## Task 6.3 - Missing File Detection

- [x] Completed on 2026-11-13. Added persisted background file verification, startup/hourly/manual passes, separate missing/inaccessible/invalid states, and a bounded File recovery listing. See the [development log](docs/development-log.md).

Implement:

- File existence checks.
- Missing status.
- Missing file listing.
- Background verification.

Do not block application startup by scanning every HDD path.

**Approval required before Task 6.4.**

## Task 6.4 - File Relinking

- [x] Completed on 2026-11-13. Added native replacement selection, PDF/hash/metadata comparison, expiring previews, cancel/confirm, confirmation-time rehash, and identity-preserving relinks with notes, bookmarks and progress retained. See the [development log](docs/development-log.md).

Implement:

- Select replacement PDF.
- Validate replacement.
- Compare metadata.
- Compare hash.
- Confirm relinking.
- Update database path.

Preserve notes, bookmarks and reading progress.

**Approval required before Task 6.5.**

## Task 6.5 - Changed File Detection

- [x] Completed on 2026-11-13. Background checks compare size and full-precision timestamps, hash changed sources, retain derivatives for identical bytes, and invalidate changed revisions while preserving manual metadata. See the [development log](docs/development-log.md).

Detect:

- Changed file size.
- Changed modification timestamp.
- Changed hash when required.

Do not overwrite user metadata.

**Approval required before Task 6.6.**

## Task 6.6 - Duplicate Management UI

- [x] Completed on 2026-11-13. Added bounded file comparisons, preferred copies, ignore/reopen, and acknowledged database-only removal protected by a freshly rehashed kept copy. See the [development log](docs/development-log.md).

Implement:

- Duplicate groups.
- File comparison.
- Keep selected record.
- Ignore duplicates.
- Remove database record.
- Safe manual review.

Removing a record must not remove the original PDF.

**Approval required before Phase 7.**

---

# PHASE 7: SETTINGS, ANALYTICS AND BACKUP

**Objective:** Complete the application management and recovery experience.

## Task 7.1 - Settings Page

- [x] Completed on 2026-11-13. Added all seven settings areas, persisted startup/library defaults, existing reader preference editing, independent appearance controls, data location/status, and clear backup/storage availability. Verified persistence, routing, errors and builds. See the [development log](docs/development-log.md).

Implement:

- General.
- Appearance.
- Library.
- Reader.
- Storage.
- Backup.
- Privacy.

**Approval required before Task 7.2.**

## Task 7.2 - Storage Management

- [x] Completed on 2026-11-13. Added categorized AppData usage and confirmed cache cleanup with protected database/backups/PDFs, nonrecursive deletion, directory guards, and a 24-hour temporary-file policy. Tests and build checks passed; see the [development log](docs/development-log.md).

Display:

- Database size.
- Thumbnail cache size.
- Temporary storage.
- Backup size.
- Cache usage.

Implement safe cache cleanup.

Never delete database records when clearing cache.

**Approval required before Task 7.3.**

## Task 7.3 - Backup Creation

- [x] Completed on 2026-11-13. Added live SQLite snapshots, versioned folder manifests, SHA-256/integrity/schema validation, optional verified thumbnails, destination selection and cancellation. Originals are excluded. Tests and builds passed; see the [development log](docs/development-log.md).

Implement:

- Database backup.
- Metadata backup.
- Settings backup.
- Optional thumbnail backup.
- Backup location selection.
- Backup validation.

Original PDFs are excluded by default.

**Approval required before Task 7.4.**

## Task 7.4 - Backup Restore

Implement:

- Backup inspection.
- Version compatibility.
- Integrity validation.
- Safety backup.
- Restore confirmation.
- Search index rebuilding.

Never silently overwrite a newer database.

**Approval required before Task 7.5.**

## Task 7.5 - Reading Analytics

Implement:

- Books completed.
- Books currently reading.
- Pages read.
- Reading activity.
- Category distribution.
- Reading history.

Use real persisted data.

**Approval required before Phase 8.**

---

# PHASE 8: QUALITY ASSURANCE AND RELEASE

**Objective:** Prepare LibraDesk for reliable everyday use on Windows 11.

## Task 8.1 - Rust Backend Testing

Test:

- File discovery.
- PDF validation.
- Metadata extraction.
- Hashing.
- Duplicate detection.
- Path handling.
- Database operations.
- File recovery.

**Approval required before Task 8.2.**

## Task 8.2 - Frontend Testing

Test:

- Routing.
- Navigation.
- Search.
- Filters.
- Forms.
- Reader controls.
- Bookmarks.
- Settings.

**Approval required before Task 8.3.**

## Task 8.3 - Integration Testing

Test:

- Import.
- Interrupted scanning.
- Resume.
- Database persistence.
- Metadata editing.
- Reader progress.
- Backup.
- Restore.
- Missing file relinking.

**Approval required before Task 8.4.**

## Task 8.4 - Large Collection Performance Testing

Use test datasets containing:

- 100 PDFs.
- 1,000 PDFs.
- 5,000 PDFs.
- 10,000+ PDF records.

Measure:

- Initial loading.
- Search performance.
- Grid scrolling.
- Import performance.
- Memory consumption.
- Database size.
- Thumbnail cache usage.

Use representative large PDF files and avoid copying the entire personal collection into the repository.

**Approval required before Task 8.5.**

## Task 8.5 - Security and Data Safety Review

Verify:

- Tauri permissions.
- Content Security Policy.
- Local file access.
- PDF JavaScript restrictions.
- Input validation.
- SQL injection prevention.
- Path traversal protection.
- Backup integrity.
- Destructive operation safeguards.

**Approval required before Task 8.6.**

## Task 8.6 - Windows Release Build

Implement:

- Application icon.
- Version information.
- Installer configuration.
- Production build.
- Installation testing.
- Uninstallation testing.
- Data preservation testing.

Verify that uninstalling LibraDesk does not delete original PDF files.

**Approval required before Task 8.7.**

## Task 8.7 - Final Documentation

Update:

- README.md.
- Installation instructions.
- User guide.
- Technical architecture.
- Database documentation.
- Backup instructions.
- Troubleshooting guide.
- Known limitations.

Create a final release report.

**Project completion requires explicit user approval.**

---

# 4. REQUIRED DOCUMENTATION MANAGEMENT

Maintain these documents throughout development:

```text
docs/
├── plan.md
├── environment-audit.md
├── architecture.md
├── database-schema.md
├── decisions.md
├── development-log.md
├── testing-strategy.md
├── user-guide.md
└── troubleshooting.md
```

## plan.md Status Management

Use these statuses:

- [ ] Not Started
- [ ] In Progress
- [x] Completed
- [!] Blocked
- [-] Skipped with User Approval

Every completed task must have:

- Completion date.
- Implementation summary.
- Test results.
- Known issues.
- Commit reference where applicable.

Only mark a task completed after verifying its acceptance criteria.

Never mark future tasks as completed.

---

# 5. FAILURE AND BLOCKER HANDLING

If a task encounters an issue:

1. Stop the current implementation.
2. Identify the root cause.
3. Report the affected functionality.
4. Explain the available solutions.
5. Recommend the safest solution.
6. Ask the user how to proceed.

Do not silently change the approved architecture.

If a dependency is incompatible:

- Explain the compatibility issue.
- Identify alternatives.
- Present the trade-offs.
- Obtain approval before replacing the dependency.

If a build fails:

- Report the exact failure.
- Investigate the cause.
- Fix only the relevant issue.
- Rerun the failed checks.
- Report the actual result.

Do not proceed to unrelated tasks while a blocking issue remains unresolved.

---

# 6. FINAL ACCEPTANCE CRITERIA

LibraDesk can be considered production-ready only when:

- [ ] Windows desktop application launches reliably.
- [ ] SQLite database initializes and persists.
- [ ] AppData storage works.
- [ ] Multiple HDD folders can be registered.
- [ ] Recursive PDF scanning works.
- [ ] Import preview works.
- [ ] Interrupted scanning can resume.
- [ ] Metadata extraction works.
- [ ] Cover thumbnails are generated.
- [ ] Duplicate detection works.
- [ ] Original PDF files remain untouched.
- [ ] Categories and collections work.
- [ ] Bengali and English search work.
- [ ] Book Details pages work.
- [ ] Built-in PDF Reader works.
- [ ] Reading progress persists.
- [ ] Bookmarks and Notes work.
- [ ] Missing files can be relinked.
- [ ] Backup and restore work.
- [ ] Large library navigation remains responsive.
- [ ] Windows installer works.
- [ ] Uninstallation preserves original books.
- [ ] Documentation is complete.
- [ ] Critical tests pass.
- [ ] Final user approval is received.

---

# 7. FIRST COMMAND TO CODEX

Start with **Phase 0, Task 0.1 only**.

Inspect the existing repository and report its current structure, dependencies, implementation status, configuration and potential issues.

Do not:

- Modify application source code.
- Install dependencies.
- Create the entire application.
- Start Phase 1.
- Execute multiple tasks.
- Skip the approval process.

After completing Task 0.1, provide the mandatory Task Completion Report and wait for my explicit approval.

**Remember: One task, one implementation, one verification, one completion report, one user approval.**

This approval workflow is mandatory throughout the entire LibraDesk project.
