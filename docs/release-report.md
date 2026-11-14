# LibraDesk 0.2.0 release report

Date: 2026-11-14. Owner-authorized original-PDF metadata write-back and manual thumbnails, with the explicit choice of atomic save without persistent original-file backups. Historical distribution evidence remains in the [0.1.1 report](releases/0.1.1-report.md).

## Delivered behavior

Manual metadata and organization edits enqueue per-file Info/XMP updates, including genre and taxonomy. A durable prepared-attempt journal reconciles interrupted replacement and newer generations. Files are staged, synced, validated and replaced on the same volume using MoveFileExW; source DACL and Zone.Identifier are preserved. Active readers defer writing. Source changes, signatures, encryption/EFS, oversized/unsupported PDFs and access failures are refused visibly. Book details show completion/failure and Retry/Stop; global notifications provide View book.

Custom PNG/JPEG/WebP thumbnails support preview/cancel/save/reset and are normalized into database BLOBs. They survive cache cleanup and all database backups and never change PDF pages. Notes/bookmarks/progress stay local and retain their anchors after metadata-only updates. Restore refuses live prepared attempts and clears restored write intents. Record removal never deletes PDFs.

## Artifact

| Item | Value |
| --- | --- |
| Target | Windows 11 x64, Intel/AMD |
| Setup | LibraDesk_0.2.0_x64-setup.exe |
| Size | 224,588,853 bytes (~214.18 MiB) |
| SHA-256 | 8b9aca0af1c59f9985cfa342e0beda6c956c379fd26aec8a46664567db9482a1 |
| Embedded prerequisite | Full WebView2 Evergreen standalone, 215,116,512 bytes; valid Microsoft signature |
| App/setup signature | Unsigned |
| Release | [v0.2.0](https://github.com/zabirarkam27/LibraDesk/releases/tag/v0.2.0) |
| Setup download | [setup.exe](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.2.0/LibraDesk_0.2.0_x64-setup.exe) |
| Integrity | [SHA256SUMS.txt](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.2.0/SHA256SUMS.txt) |

## Verification

- 56 ordinary Rust tests passed; three explicit helpers/benchmark ignored in the routine run.
- Eleven browser suites passed, including source-write states, failure notification/navigation, genre and real image preview/cancel/save/reset.
- Frontend production build, Rust fmt and Clippy with warnings denied passed.
- Actual isolated native IPC edited a generated PDF and saved a JPEG-derived cover, retained it through cache cleanup, created a backup and rendered the edited PDF in the actual reader.
- Independent post-close verification reopened the disk PDF and checked standard/custom fields, original page bytes, notes and database integrity.
- Native regressions cover crash after replacement, newer edits, locked retry, tampered staging, cancellation, signatures/encryption/oversize, read-only/external changes, active readers, Windows DACL/origin preservation, restore blocking and custom-cover backup/restore.
- Final installer payload/runtime signature audited; no PDF or SQLite library payload packaged.
- Real current-user installation, expected executable bytes/version, authorized release startup/native IPC/settings/offline navigation and uninstall passed. Source PDF/data fixtures and registry cleanup checks passed.

Evidence: [native edits](qa/source-edits-native-0.2.0.json), [distribution](qa/distribution-0.2.0.json), [installer](qa/installer-0.2.0.json), [release smoke](qa/release-smoke-0.2.0.json). Publication receipt is recorded after upload.

## Limits

No personal PDF was edited for QA. The installed release smoke used the previously authorized live AppData launch without metadata-edit/restore/deletion commands. Merely upgrading does not queue PDF writes. A runtime-absent fresh OS was not tested. Historical 10,001-record idle-catalog performance does not measure sustained source writing.

Metadata writes support writable unsigned/unencrypted PDFs up to 256 MiB and require staging space. No persistent PDF backup or source-metadata undo is supplied. Incremental revisions can retain previous metadata bytes; PDF/A conformance and preservation of every filesystem attribute/alternate stream are not certified. Custom fields may not display in other viewers or reconstruct on fresh import. Database restore cannot undo physical PDF edits.

Final project acceptance remains separate from authorization to implement and distribute this feature.
