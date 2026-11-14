import { useEffect } from "react";
import { useNavigate } from "react-router";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { desktop } from "@/shared/lib/preferences-client";

type FailedWrite = { book: number; title: string; message: string };
export function PdfWriteNotifications() {
  const navigate = useNavigate();
  useEffect(() => {
    if (!desktop) return;
    const listening = listen<FailedWrite>("pdf-write-failed", ({ payload }) => {
      toast.error("PDF metadata was not saved", {
        id: "pdf-write-failed-" + payload.book,
        description: payload.title + ": " + payload.message,
        duration: 12000,
        action: { label: "View book", onClick: () => navigate("/library/" + payload.book) },
      });
    }).catch(() => undefined);
    return () => { void listening.then(remove => remove?.()); };
  }, [navigate]);
  return null;
}
