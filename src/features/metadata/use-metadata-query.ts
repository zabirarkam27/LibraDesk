import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { metadataEvent } from "./client";

export function useMetadataQuery<T>(query: () => Promise<T>) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState(false);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let disposed = false;
    let loading = false;
    let dirty = false;
    let timer: number | undefined;
    setData(null);
    setError(false);

    function schedule() {
      if (disposed || timer !== undefined) return;
      if (loading) { dirty = true; return; }
      timer = window.setTimeout(() => {
        timer = undefined;
        loading = true;
        void query().then(
          (result) => { if (!disposed) { setData(result); setError(false); } },
          () => { if (!disposed) setError(true); },
        ).finally(() => {
          loading = false;
          if (dirty) { dirty = false; schedule(); }
        });
      }, 200);
    }

    schedule();
    const subscriptions = [metadataEvent, "scan-updated", "catalog-updated"].map((event) =>
      listen(event, schedule).catch(() => undefined));
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      void Promise.all(subscriptions).then((remove) => remove.forEach((unlisten) => unlisten?.()));
    };
  }, [query, attempt]);

  return { data, error, retry: () => setAttempt((value) => value + 1) };
}
