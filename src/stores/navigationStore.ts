import { createSignal } from "solid-js";
import {
  searchBookmarks,
  searchHistory,
  type Bookmark,
  type HistoryEntry,
} from "../lib/ipc";

export interface Suggestion {
  type: "bookmark" | "history";
  url: string;
  title: string;
}

const [suggestions, setSuggestions] = createSignal<Suggestion[]>([]);
const [showSuggestions, setShowSuggestions] = createSignal(false);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function fetchSuggestions(query: string): void {
  if (debounceTimer) clearTimeout(debounceTimer);

  if (!query.trim()) {
    setSuggestions([]);
    setShowSuggestions(false);
    return;
  }

  debounceTimer = setTimeout(async () => {
    try {
      const [bookmarks, history] = await Promise.all([
        searchBookmarks(query).catch(() => [] as Bookmark[]),
        searchHistory(query).catch(() => [] as HistoryEntry[]),
      ]);

      const seen = new Set<string>();
      const merged: Suggestion[] = [];

      // Bookmarks first (higher priority)
      for (const bm of bookmarks) {
        if (!seen.has(bm.url) && merged.length < 8) {
          seen.add(bm.url);
          merged.push({ type: "bookmark", url: bm.url, title: bm.title });
        }
      }

      // Then history entries
      for (const h of history) {
        if (!seen.has(h.url) && merged.length < 8) {
          seen.add(h.url);
          merged.push({ type: "history", url: h.url, title: h.title });
        }
      }

      setSuggestions(merged);
      setShowSuggestions(merged.length > 0);
    } catch {
      setSuggestions([]);
      setShowSuggestions(false);
    }
  }, 200);
}

function clearSuggestions(): void {
  if (debounceTimer) clearTimeout(debounceTimer);
  setSuggestions([]);
  setShowSuggestions(false);
}

export {
  suggestions,
  showSuggestions,
  fetchSuggestions,
  clearSuggestions,
};
