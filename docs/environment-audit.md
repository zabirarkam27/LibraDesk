# Windows development environment audit

Checked on 2026-11-10 against scaffold commit `276b379`. The installed tools can build the current Windows scaffold.

| Item | Verified on this machine |
| --- | --- |
| Windows | Windows 11 Pro 25H2, x64, build 26200.9711 |
| Node.js and npm | Node.js 24.15.0 satisfies the manifest's Node range. npm 11.12.1 is installed. |
| Rust and Cargo | `stable-x86_64-pc-windows-msvc` is active. Both rustc and Cargo report 1.96.1. |
| Tauri | CLI and JS API 2.12.4, Rust crate 2.12.5. Tauri reports React, Vite, and the expected loopback development URL and frontend output. |
| WebView2 | Runtime 157.0.4341.42 detected by Tauri. |
| C++ build tools | Visual Studio Build Tools 2022, installation 17.14.37411.7, with MSVC 14.44.35207. |
| Windows SDK | Include directory for SDK 10.0.26100.0 is installed. |
| Git | Git for Windows 2.54.0. The workspace tracks the configured GitHub remote. |

`npm.cmd run build` and `cargo check --offline --locked --manifest-path src-tauri/Cargo.toml` both passed during the audit. The scaffold's earlier `npm.cmd run tauri build -- --debug --no-bundle` passed and its executable was still present. The earlier desktop smoke check returned the Rust IPC response in the webview. Release installer tooling is verified in Task 8.6.

No missing or incompatible prerequisite was found for the current build. Tauri's optional upstream crate-version lookups failed in the restricted shell. The local version and environment checks completed. Cargo fell back to copying incremental cache files when hard links were unavailable in the sandbox. Compilation succeeded.

[Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)
