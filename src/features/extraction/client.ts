import { invoke } from "@tauri-apps/api/core";
export type JobKey = { fileId: number; revision: number; lease: number };
export type TextTask = JobKey & { length: number; nextPage: number; totalPages: number | null };
export type PageText = { number: number; text: string };
export type ExtractionStatus = {
  revision: number; status: "queued" | "running" | "completed" | "failed"; completedPages: number;
  totalPages: number | null; pagesWithText: number; error: string | null;
};
export type Failure = "password_required" | "invalid_pdf" | "source_unavailable" | "page_text_limit" | "extraction_timeout" | "extraction_failed";
export const claim = () => invoke<TextTask | null>("claim_text_extraction");
export const read = (key: JobKey, offset: number, length: number) => invoke<ArrayBuffer>("read_extraction_range", { key, offset, length });
export const renew = (key: JobKey) => invoke<boolean>("renew_text_extraction", { key });
export const commit = (key: JobKey, total: number, pages: PageText[]) => invoke<boolean>("commit_page_text", { key, total, pages });
export const release = (key: JobKey) => invoke<void>("release_text_extraction", { key });
export const fail = (key: JobKey, reason: Failure) => invoke<void>("fail_text_extraction", { key, reason });
export const retry = (file: number, revision: number) => invoke<boolean>("retry_text_extraction", { file, revision });
export const getStatus = (file: number) => invoke<ExtractionStatus | null>("text_extraction_status", { file });
