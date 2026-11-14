# Installation

LibraDesk 0.2.0 targets Windows 11 x64 (Intel/AMD). Download [LibraDesk_0.2.0_x64-setup.exe](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.2.0/LibraDesk_0.2.0_x64-setup.exe) from the public release. No developer tools are needed. The current-user NSIS installer includes Microsoft's full WebView2 Evergreen standalone installer and supplies the runtime when it is missing. Once setup is downloaded, the missing-runtime installation does not need a separate internet download. Existing runtimes are reused. Organization policies may restrict installation.

Suggested planning minimum: a Windows 11-compatible x64 dual-core CPU, 8 GB RAM, 2 GB free working space (plus PDFs/cache/backups), and a 1280 × 720 display. Prefer 16 GB RAM, 5 GB free space and SSD app storage for a large library. These are practical recommendations, not hardware limits measured on every configuration; testing used a Ryzen 5 8500G and about 32 GB RAM. No ARM64, 32-bit, macOS or Linux installer is published.

Build the installer from a checkout with Node/npm, Rust MSVC, Visual Studio C++ build tools and the Windows SDK:

~~~powershell
npm ci
npm run tauri build -- --bundles nsis -- --locked
~~~

Output: src-tauri/target/release/bundle/nsis/LibraDesk_0.2.0_x64-setup.exe. Rust packages, NSIS and the Microsoft standalone runtime must be cached or downloadable during development. Core application use needs no network. The produced application and installer are unsigned; Windows may show an unknown-publisher/SmartScreen prompt. Verify the official release and its [SHA256SUMS.txt](https://github.com/zabirarkam27/LibraDesk/releases/download/v0.2.0/SHA256SUMS.txt) before installing. Do not disable security software to run the installer.

Run the installer and select the destination. The default current-user destination is %LOCALAPPDATA%\LibraDesk. Library data lives in its database, thumbnails, cache, backups, logs and temp subfolders. A custom installation destination does not relocate that library data.

Open LibraDesk, then Import to register folders and review a scan. See the [user guide](user-guide.md). Close a running copy before installing a new build. Downgrades through silent/update installation are refused; backups created by newer versions are also refused by restore.

Uninstall through Windows Installed apps or uninstall.exe. Normal uninstallation removes application files and shortcuts, retains the library data, and never deletes the original PDFs. Keep a complete backup before upgrading or manually deleting AppData. Removing a library record and removing an application installation are different operations; neither is a source-PDF deletion command.

The release QA uses a clean installation directory on the existing Windows account, plus separate generated-data native profile evidence. The embedded Microsoft runtime has a valid Microsoft signature and the NSIS payload is checked for its inclusion. A fresh Windows account/VM with WebView2 absent was not part of this run. See [Tauri offline-installer packaging](https://v2.tauri.app/distribute/windows-installer/#offline-installer) and [Microsoft offline runtime deployment](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution#offline-deployment).
