import { getCurrentWindow } from "@tauri-apps/api/window";
import { flushReaderBeforeClose } from "./reader-client";
import { toast } from "sonner";

export function guardReaderWindowClose() {
  const window = getCurrentWindow();
  let closing = false;
  return window.onCloseRequested(async (event) => {
    event.preventDefault();
    if (closing) return;
    closing = true;
    try { await flushReaderBeforeClose(); await window.destroy(); }
    catch { closing = false; toast.error("Reading progress could not be saved. Try closing the window again."); }
  });
}
