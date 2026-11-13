import { spawnSync } from "node:child_process";
const suites = ["4","5","61","62","6-recovery","71","72","73","74","75"];
for (const suite of suites) {
  console.log("\nBrowser suite " + suite);
  const result = spawnSync(process.execPath, ["tests/browser/phase" + suite + "-ui-check.mjs"], { stdio: "inherit", windowsHide: true });
  if (result.status !== 0) process.exit(result.status || 1);
}
