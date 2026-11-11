import { invoke } from "@tauri-apps/api/core";
export type ReaderPreferences = {
  zoom: "fit_width" | "fit_page" | "custom"; scale: number;
  layout: "single" | "continuous"; fullscreen: boolean; shortcuts: boolean;
};
export type ReaderDocument = {
  sessionId: number; bookId: number; fileId: number; revision: number; length: number;
  title: string; resumePage: number; staleProgress: boolean; preferences: ReaderPreferences;
};
export type ReaderAnnotation = {
  id: number; fileId: number | null; fileRevision: number; page: number; text: string; revision: number;
};
export type AnnotationKind = "bookmark" | "note";
export type Annotations = { bookmarks: ReaderAnnotation[]; notes: ReaderAnnotation[] };
export type Progress = { sessionId: number; sequence: number; page: number; total: number; activeMs: number; close: boolean };
export const openReader = (book: number, file: number | null) => invoke<ReaderDocument>("open_reader", { book, file });
export const readRange = (session: number, offset: number, length: number) =>
  invoke<ArrayBuffer>("read_reader_range", { session, offset, length });
export const saveProgress = (input: Progress) => invoke<void>("save_reader_progress", { input });
export const closeReader = (session: number) => invoke<void>("close_reader", { session });
export const saveReaderPreferences = (input: ReaderPreferences) => invoke<void>("save_reader_preferences", { input });
export const getAnnotations = (book: number) => invoke<Annotations>("reader_annotations", { book });
export const saveAnnotation = (kind: AnnotationKind, input: {
  sessionId: number; id: number | null; revision: number | null; page: number; text: string; clientKey: string;
}) => invoke<number>("save_reader_annotation", { kind, input });
export const deleteAnnotation = (kind: AnnotationKind, session: number, id: number, revision: number) =>
  invoke<void>("delete_reader_annotation", { kind, session, id, revision });

// Opening and finishing readers share one queue so a late close cannot overwrite a newer book.
let lifecycle: Promise<unknown> = Promise.resolve();
export function readerLifecycle<T>(operation: () => Promise<T>): Promise<T> {
  const next = lifecycle.then(operation);
  lifecycle = next.catch(() => undefined);
  return next;
}
let shutdown: (() => Promise<void>) | null = null;
export function registerReaderShutdown(operation: () => Promise<void>) {
  shutdown = operation;
  return () => { if (shutdown === operation) shutdown = null; };
}
export const flushReaderBeforeClose = () => shutdown?.() ?? Promise.resolve();
