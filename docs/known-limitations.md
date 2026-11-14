# Known limitations · 0.2.0

- Windows 11 x64 is the release target tested here. The installer now includes the full WebView2 Evergreen standalone prerequisite and installs it if missing; a fresh OS without it was not tested. Existing runtimes should be kept updated. The installer/executable are unsigned. ARM64/32-bit Windows, macOS and Linux builds are not published.
- Offline local use is supported. Cloud sync, accounts, OCR and scheduled backups are outside this version.
- Metadata and PDF page-content indexing run incrementally. Search coverage can be partial. Encrypted files require manual reader passwords and cannot be indexed in the background.
- Source PDFs remain external to the database and backups. A database restore cannot recover a missing original PDF; preserve source files independently.
- Per-page extracted text is capped at 256 KiB and fails explicitly above that limit. Native metadata parsing is bounded to 32 MiB sources; larger documents use frontend processing where supported. Complex/malformed PDFs may fail or consume more resources than ordinary library use.
- Reader canvases and images are bounded, but a large image PDF can still use more memory than the normal idle-catalog target. The 500 MiB result is an idle 10,001-record catalog measurement with parsing queues removed, not a universal limit.
- Analytics count saved page views per session and active visible time, not comprehension or unique lifetime pages. Activity is attributed to a session's start day; a session spanning midnight is not split.
- Restoring retains older cover generations for safety. Thumbnail cleanup removes generated images, while protected folders and safety backups remain until deliberately managed.
- Backup SHA-256 verifies integrity, not trusted authorship. Backups contain private library data and extracted text without encryption.
- Same-size scan matches are duplicate candidates only. Exact duplicate review requires hashes. Database record deletion also cascades library annotations for a book's last file, while leaving PDFs intact.
- Benchmarks use generated one-page files on this development machine; source-drive latency, antivirus, complex PDFs and different hardware affect results. QA combines real backend/native tests with mock-IPC browser tests; it does not certify every Windows setup or physical disk.

- Manual metadata saves change original PDFs in v0.2.0. No persistent original backup is created. Atomic replacement requires write permissions, same-folder staging space and a supported unsigned, unencrypted PDF up to 256 MiB. EFS-encrypted source files are refused. Locked/external-change failures remain visible and require attention; catalog success alone does not confirm a PDF update.
- Notes, progress and custom thumbnails remain library data. Custom metadata uses LibraDesk namespaces and is not universally displayed or fully reconstructed by a fresh import.
- Incremental saves retain original page content and may retain previous metadata bytes. They do not guarantee secure erasure, archival/PDF-A conformity, preservation of every filesystem attribute/alternate stream or exact-hash duplicate grouping. Source DACL and Zone.Identifier are preserved on the tested Windows filesystem.
- Stop is not undo. Database restore cannot undo original-PDF edits and will not replay restored source-write jobs. Finish/stop prepared saves before restoring or removing their file records.
- Performance budgets were measured on the earlier idle catalog, not sustained v0.2.0 PDF writing.
