# Known limitations · 0.1.1

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
