# Performance report · 2026-11-14

Environment: Windows 11 Pro 10.0.26200, AMD Ryzen 5 8500G, 33,464,832,000 bytes RAM (~31.2 GiB). Local generated fixtures; debug Rust/native measurements and production-built frontend assets. Background/system activity and OS caches affect results.

The owner accepted warm search ≤500 ms, initial UI ≤3 seconds and normal-library memory ≤500 MiB before benchmarking.

| Records | Discovery ms | Confirmed import ms | DB reopen + first query ms | Warm metadata search max ms | Database bytes | Thumbnail bytes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 24.89 | 69.96 | 4.25 | 1.55 | 573,440 | 0 |
| 1,000 | 161.47 | 597.96 | 5.02 | 11.20 | 1,867,776 | 0 |
| 5,000 | 889.37 | 4,421.41 | 8.24 | 54.78 | 7,610,368 | 0 |
| 10,001 | 2,438.20 | 9,787.29 | 11.88 | 133.55 | 14,790,656 | 0 |

The scale set contains valid generated one-page PDFs, using unique paths and bilingual filenames. Import measures discovery/header validation and transactional records; asynchronous metadata/cover/text extraction is excluded. Each search has a warm-up and ten samples for common Bengali, partial and punctuation/SQL-like text. The table reports the slowest observed query sample. Database totals include scan history and queues at the measurement point. Thumbnail bytes are measured as zero because this workload does not render covers, not an estimate of a fully cached library.

Native end-to-end 10,001-record idle-catalog check:

| Metric | Observed | Budget | Result |
| --- | ---: | ---: | --- |
| Process launch to overview with real count | 705 ms | 3,000 ms | Pass |
| Warm query including IPC round trip, max of 10 | 91.91 ms | 500 ms | Pass |
| Total app + descendant WebView2 working set, max of 5 | 457.91 MiB | 500 MiB | Pass |
| Native backup snapshot count | 10,001 | 10,001 | Pass |

This native run uses the embedded-frontend debug executable and a separate generated AppData profile, with parsing queues removed. It includes native verification work and the WebView process tree. It is not a promise that large PDF rendering or simultaneous heavy indexing stays below 500 MiB. Earlier repeated native runs observed 459–461 MiB and 0.7–1.28 seconds startup.

Separate headless-Chrome frontend lab (fixture IPC, real production frontend):

| Records | First visible grid ms | Scroll frame p50 / p95 ms | Rendered items | JS heap bytes |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 580.69 | 16.5 / 19.3 | 12 | 7,941,828 |
| 1,000 | 357.02 | 16.5 / 19.4 | 12 | 7,641,380 |
| 5,000 | 391.80 | 16.7 / 19.5 | 12 | 14,374,868 |
| 10,001 | 379.22 | 16.7 / 18.8 | 12 | 20,000,508 |

Scroll samples cover 60 alternating top/bottom animation frames. Polling adds up to about 100 ms to first-visible timing. The browser fixture allocates its full dataset in JavaScript, unlike production's 50-record query pages. JS heap excludes browser/native memory.

Real PDF.js regressions opened a 2,000-page document, navigated rapidly, bounded canvases/thumbnails and released PDF workers when leaving the reader. A sparse source over 300 MiB opened using 212,733 range bytes. Password, invalid text, extraction limits and malformed PDFs were exercised. These reader checks are separate from the collection-import workload.

Reproduce with [testing strategy](testing-strategy.md). Reviewed raw snapshots are under docs/qa; generated sources and local profiles remain ignored under target. HDD seek latency, source-file complexity, antivirus and sustained thumbnail/indexing workloads need collection-specific measurement. No personal collection was copied into Git.
