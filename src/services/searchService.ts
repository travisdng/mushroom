import { call } from "./ipc";
import type { IndexProgress, IndexStats, SearchResults } from "../types/search";

export function searchNotes(
  query: string,
  folder: string | null,
  modifiedAfter: number | null,
  limit = 50,
): Promise<SearchResults> {
  return call<SearchResults>("search_notes", {
    query,
    folder,
    modifiedAfter,
    limit,
  });
}

export function getIndexStats(): Promise<IndexStats> {
  return call<IndexStats>("get_index_stats");
}

export function rebuildIndex(): Promise<IndexProgress> {
  return call<IndexProgress>("rebuild_index");
}

export function cancelRebuild(): Promise<void> {
  return call<void>("cancel_rebuild");
}
