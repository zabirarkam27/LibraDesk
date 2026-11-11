import { invoke } from "@tauri-apps/api/core";
import type { BookSummary } from "./catalog-client";

export type ReadingStatus = "unread" | "reading" | "completed" | "on_hold";
export const readingStatuses: { value: ReadingStatus; label: string }[] = [
  { value: "unread", label: "Unread" }, { value: "reading", label: "Reading" },
  { value: "completed", label: "Completed" }, { value: "on_hold", label: "On Hold" },
];
export type GroupKind = "category" | "tag" | "collection";
export type Group = {
  id: number; name: string; parentId: number | null; description: string | null;
  sortMode: "manual" | "title" | "added"; revision: number; count: number;
};
export type Organization = { categories: Group[]; tags: Group[]; collections: Group[]; languages: string[] };
export function categoryPath(group: Group, categories: Group[]) {
  const names = [group.name];
  const visited = new Set([group.id]);
  let parent = group.parentId;
  const byId = new Map(categories.map((category) => [category.id, category]));
  while (parent !== null && !visited.has(parent)) {
    const category = byId.get(parent);
    if (!category) break;
    visited.add(parent);
    names.unshift(category.name);
    parent = category.parentId;
  }
  return names.join(" / ");
}
export type GroupInput = {
  id?: number; revision?: number; name: string; parentId?: number | null;
  description?: string | null; sortMode?: Group["sortMode"];
};
export type CatalogQuery = {
  text?: string; page?: number; sort?: "title" | "author" | "added" | "size" | "year" | "progress" | "relevance" | "collection";
  direction?: "asc" | "desc"; categoryId?: number | null; tagId?: number | null;
  collectionId?: number | null; language?: string | null; readingStatus?: ReadingStatus | null; favoritesOnly?: boolean;
};
export type CatalogPage = { books: BookSummary[]; total: number; page: number };
export type Dashboard = {
  totalBooks: number; totalStorage: number; totalPages: number; favorites: number; reading: number;
  categories: { id: number; name: string; count: number }[];
  recentlyAdded: BookSummary[]; recentlyRead: BookSummary[];
  favoriteBooks: BookSummary[]; currentlyReading: BookSummary[];
};
export const queryCatalog = (query: CatalogQuery) => invoke<CatalogPage>("query_catalog", { query });
export const getOrganization = () => invoke<Organization>("get_organization");
export const saveGroup = (kind: GroupKind, input: GroupInput) => invoke<number>("save_group", { kind, input });
export const deleteGroup = (kind: GroupKind, id: number, revision: number) => invoke<void>("delete_group", { kind, id, revision });
export const setBookFlags = (id: number, revision: number, favorite?: boolean, status?: ReadingStatus) =>
  invoke<void>("set_book_flags", { id, revision, favorite, status });
export const getDashboard = () => invoke<Dashboard>("get_dashboard");
export const editCollectionMember = (id: number, revision: number, book: number, action: "up" | "down" | "remove") =>
  invoke<void>("edit_collection_member", { id, revision, book, action });
