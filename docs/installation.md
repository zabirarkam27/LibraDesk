# Installation

LibraDesk 0.1.0 targets Windows 11 x64. The current-user NSIS installer needs no administrator installation. Microsoft Edge WebView2 Runtime must already be installed; the offline installer does not download it. Windows 11 normally supplies it. If startup reports a missing runtime, install Microsoft's WebView2 Evergreen Runtime and retry.

Build the installer from a checkout with Node/npm, Rust MSVC, Visual Studio C++ build tools and the Windows SDK:

~~~powershell
npm ci
npm run tauri build -- --bundles nsis -- --locked
~~~

Output: src-tauri/target/release/bundle/nsis/LibraDesk_0.1.0_x64-setup.exe. Rust packages, NSIS and WebView tooling must be cached or downloadable during development. Core application use needs no network. The produced application and installer are unsigned; Windows may show an unknown-publisher/SmartScreen prompt. Verify the release checksum and source before installing.

Run the installer and select the destination. The default current-user destination is %LOCALAPPDATA%\LibraDesk. Library data lives in its database, thumbnails, cache, backups, logs and temp subfolders. A custom installation destination does not relocate that library data.

Open LibraDesk, then Import to register folders and review a scan. See the [user guide](user-guide.md). Close a running copy before installing a new build. Downgrades through silent/update installation are refused; backups created by newer versions are also refused by restore.

Uninstall through Windows Installed apps or uninstall.exe. Normal uninstallation removes application files and shortcuts, retains the library data, and never deletes the original PDFs. Keep a complete backup before upgrading or manually deleting AppData. Removing a library record and removing an application installation are different operations; neither is a source-PDF deletion command.

The release QA used a clean installation directory on an existing Windows account, plus a separate generated-data native profile. A fresh Windows account/VM with WebView2 absent was not part of this run.
