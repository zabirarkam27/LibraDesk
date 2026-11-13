import { invoke } from "@tauri-apps/api/core";

export type RawMetadata = {
  title: string | null;
  author: string | null;
  subject: string | null;
  creator: string | null;
  producer: string | null;
  creationDateRaw: string | null;
  pageCount: number | null;
};

export type MetadataRow = {
  id: number;
  fileId: number;
  title: string;
  author: string | null;
  subject: string | null;
  path: string;
  raw: RawMetadata;
  needsReview: boolean;
  pending: boolean;
  error: string | null;
};

export type Edit = { field: "title" | "author" | "subject"; value: string };
export type DuplicateGroup = {
  hash: string;
  count: number;
  reviewed: boolean;
  members: { title: string; path: string }[];
};

export type CoverTask = { fileId: number; revision: number; generation:number; length: number; needsMetadata: boolean };
export type CoverResult = { png: number[] | null; raw: RawMetadata | null };

export const metadataEvent = "metadata-updated";
export const metadataPage = (before: number | null, missingOnly: boolean) =>
  invoke<MetadataRow[]>("metadata_review_page", { before, missingOnly });
export const saveMetadata = (ids: number[], edits: Edit[]) => invoke("save_book_metadata", { ids, edits });
export const listDuplicates = (after: string | null) => invoke<DuplicateGroup[]>("list_duplicate_groups", { after });
export const setDuplicateReview = (hash: string, reviewed: boolean) => invoke("set_duplicate_review", { hash, reviewed });
export const nextCoverTask = () => invoke<CoverTask | null>("next_cover_task");
export const finishCover = (task: CoverTask, result: CoverResult) =>
  invoke("finish_cover", { fileId: task.fileId, revision: task.revision, generation:task.generation, ...result });
export const getCover = (fileId: number, revision: number) => invoke<ArrayBuffer>("get_book_cover", { fileId, revision });
export const coverRange = (task: CoverTask, offset: number, length: number) =>
  invoke<ArrayBuffer>("read_cover_range", { fileId: task.fileId, revision: task.revision, generation:task.generation, offset, length });
