import { finishCover, nextCoverTask } from "./client";
import { desktop } from "@/shared/lib/preferences-client";
import { beginBackgroundWork } from "@/shared/lib/background-work";

let started = false;

export function startCoverWorker() {
  if (!desktop || started) return;
  started = true;

  async function run() {
    let interval = 1200;
    const finish=beginBackgroundWork();
    try {
      if(!finish)return;
      const task = await nextCoverTask();
      if (task) {
        const render = await import("./render-cover").catch(() => null);
        const fallback = { png: null, raw: null };
        const output = render && task.length <= render.MAX_SOURCE_BYTES
          ? await render.renderCover(task).catch(() => fallback)
          : fallback;
        await finishCover(task, output);
        interval = 50;
      }
    } catch {
      interval = 3000;
    } finally {
      finish?.();
      window.setTimeout(() => void run(), interval);
    }
  }

  void run();
}
