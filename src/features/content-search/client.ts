import { invoke } from "@tauri-apps/api/core";
export type PageHit = {
  id: number; bookId: number; title: string; author: string | null; fileId: number;
  revision: number; path: string; pageNumber: number; snippet: string;
};
export type PageResults = {
  hits: PageHit[]; total: number; page: number;
  coverage: { files: number; completedFiles: number; failedFiles: number; extractedPages: number };
};
export const searchPages = (text: string, page: number) => invoke<PageResults>("search_page_text", { query: { text, page } });
export const resultAddress = (hit: PageHit) => "/reader?" + new URLSearchParams({
  book: String(hit.bookId), file: String(hit.fileId), page: String(hit.pageNumber), revision: String(hit.revision),
});
