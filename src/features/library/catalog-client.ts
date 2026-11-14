import { invoke } from "@tauri-apps/api/core";
import type { ReadingStatus } from "./library-client";

export type Author = { id: number; displayName: string };

export type BookFile = {
  id: number;
  bookId: number;
  path: string;
  bytes: number;
  modifiedAt: number;
  lastVerifiedAt: number | null;
  pages: number | null;
  availability: "available" | "missing" | "inaccessible" | "invalid";
  revision: number;
  isPrimary: boolean;
  coverRevision: number | null;
};

export type BookSummary = {
  id: number;
  title: string;
  path: string | null;
  bytes: number | null;
  fileId: number | null;
  author: string | null;
  pages: number | null;
  coverRevision: number | null;
  duplicate: boolean;
  isFavorite: boolean;
  readingStatus: ReadingStatus;
  revision: number;
  language: string | null;
  publicationYear: number | null;
};

export type BookInput = {
  title: string;
  subject?: string | null;
  genre?: string | null;
  publisher?: string | null;
  isbn?: string | null;
  publicationYear?: number | null;
  language?: string | null;
  // Omit on update to preserve the current authors. Pass [] to clear them.
  authorIds?: number[];
  authorNames?: string[];
  isFavorite?: boolean;
  readingStatus?: ReadingStatus;
  categoryIds?: number[];
  tagIds?: number[];
  collectionIds?: number[];
};

export type BookDetails = {
  id: number;
  title: string;
  author: string | null;
  subject: string | null;
  genre: string | null;
  customCoverRevision: number | null;
  publisher: string | null;
  isbn: string | null;
  publicationYear: number | null;
  language: string | null;
  revision: number;
  authors: Author[];
  files: BookFile[];
  isFavorite: boolean;
  readingStatus: ReadingStatus;
  categoryIds: number[];
  tagIds: number[];
  collectionIds: number[];
};

export const listBooks = (before: number | null = null) =>
  invoke<BookSummary[]>("list_books", { before });
export const getBook = (id: number) => invoke<BookDetails>("get_book", { id });
export const createBook = (input: BookInput) => invoke<BookDetails>("create_book", { input });
export const updateBook = (id: number, revision: number, input: BookInput) =>
  invoke<BookDetails>("update_book", { id, revision, input });
export const deleteBook = (id: number, revision: number) =>
  invoke<void>("delete_book", { id, revision });
export const getBookFile = (id: number) => invoke<BookFile>("get_book_file", { id });
export const createAuthor = (displayName: string) => invoke<Author>("create_author", { displayName });
export const listAuthors = (after: number | null = null) =>
  invoke<Author[]>("list_authors", { after });
