import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const pdfResources = ["cmaps", "standard_fonts", "wasm", "iccs"];
const pdfRoot = fileURLToPath(new URL("./node_modules/pdfjs-dist/", import.meta.url));

export default defineConfig({
  plugins: [react(), tailwindcss(), {
    name: "bundled-pdf-resources",
    async generateBundle() {
      for (const directory of pdfResources) {
        for (const file of await readdir(join(pdfRoot, directory), { withFileTypes: true })) {
          if (file.isFile()) this.emitFile({
            type: "asset", fileName: `pdfjs/${directory}/${file.name}`,
            source: await readFile(join(pdfRoot, directory, file.name)),
          });
        }
      }
    },
    configureServer(server) {
      server.middlewares.use("/pdfjs", (request, response, next) => {
        const [directory, name, extra] = (request.url ?? "").split("?")[0].split("/").filter(Boolean);
        if (!pdfResources.includes(directory) || !name || name === ".." || extra || name.includes("\\")) return next();
        void readFile(join(pdfRoot, directory, name)).then((bytes) => {
          response.setHeader("Content-Type", name.endsWith(".wasm") ? "application/wasm" : name.endsWith(".js") ? "text/javascript" : "application/octet-stream");
          response.end(bytes);
        }).catch(() => next());
      });
    },
  }],
  resolve: { tsconfigPaths: true },
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
