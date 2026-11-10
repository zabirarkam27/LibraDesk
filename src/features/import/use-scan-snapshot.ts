import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { getScanSnapshot, scanError, type ScanSnapshot } from "./scan-client";

export function useScanSnapshot(before: number | null) {
  const [data, setData] = useState<ScanSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const refresh = useRef<() => void>(() => {});

  useEffect(() => {
    let closed = false;
    let pending = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    async function load() {
      timer = undefined;
      try {
        const snapshot = await getScanSnapshot(before);
        if (!closed) {
          setData(snapshot);
          setError(null);
        }
      } catch (failure) {
        if (!closed) setError(scanError(failure));
      } finally {
        pending = false;
      }
    }

    function schedule() {
      if (closed || pending) return;
      pending = true;
      timer = setTimeout(() => void load(), 200);
    }

    refresh.current = schedule;
    schedule();
    const subscription = listen("scan-updated", schedule).catch(() => undefined);
    const fallback = setInterval(schedule, 2500);
    return () => {
      closed = true;
      clearTimeout(timer);
      clearInterval(fallback);
      void subscription.then((unlisten) => unlisten?.());
    };
  }, [before]);

  return { data, error, refresh: () => refresh.current() };
}
