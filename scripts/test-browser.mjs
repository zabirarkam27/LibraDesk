import { spawnSync } from "node:child_process";
const suites = ["phase4","phase5","phase61","phase62","phase6-recovery","phase71","phase72","phase73","phase74","phase75","source-edits"];
for (const suite of suites) {
  console.log("\nBrowser suite " + suite);
  const result = spawnSync(process.execPath, ["tests/browser/" + suite + "-ui-check.mjs"], { stdio: "inherit", windowsHide: true });
  if (result.status !== 0) process.exit(result.status || 1);
}
