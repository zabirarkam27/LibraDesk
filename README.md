# LibraDesk

LibraDesk is a Windows desktop project for managing and reading a personal PDF library offline. The current scaffold uses Tauri 2, Rust, React, TypeScript, and Vite. See [the project plan](plan.md) for the development roadmap.

## Local development

Use Node.js 20.19+ on the 20.x line or 22.12+, npm, and the Rust MSVC toolchain. Windows builds also require the Microsoft C++ build tools and WebView2. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for setup instructions.

```powershell
npm ci
npm run tauri dev
```

In the desktop window, select **Check desktop connection** to verify that React can invoke a Rust command. Run `npm run dev` for the browser preview.

## Build checks

```powershell
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --debug --no-bundle
```

The last command builds a debug desktop executable. Installer packaging is tracked in the project plan.
