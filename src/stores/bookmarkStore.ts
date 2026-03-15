import { createStore } from "solid-js/store";
import {
  addBookmark as ipcAddBookmark,
  removeBookmark as ipcRemoveBookmark,
  getBookmarks as ipcGetBookmarks,
  type Bookmark,
} from "../lib/ipc";

interface BookmarkState {
  bookmarks: Bookmark[];
  currentFolder: string | null;
  initialized: boolean;
}

const [bookmarkState, setBookmarkState] = createStore<BookmarkState>({
  bookmarks: [],
  currentFolder: null,
  initialized: false,
});

async function initializeBookmarkStore(): Promise<void> {
  if (bookmarkState.initialized) return;

  try {
    const bookmarks = await ipcGetBookmarks();
    setBookmarkState({ bookmarks, initialized: true });
  } catch (e) {
    console.error("Failed to initialize bookmark store:", e);
    setBookmarkState("initialized", true);
  }
}

async function addBookmarkAction(
  url: string,
  title: string,
  folder?: string
): Promise<void> {
  try {
    const bm = await ipcAddBookmark(url, title, folder);
    // Add to local store if it belongs in the current view
    if ((bm.folder ?? null) === bookmarkState.currentFolder) {
      setBookmarkState("bookmarks", (prev) => [bm, ...prev]);
    }
  } catch (e) {
    console.error("Failed to add bookmark:", e);
  }
}

async function removeBookmarkAction(id: string): Promise<void> {
  try {
    await ipcRemoveBookmark(id);
    setBookmarkState("bookmarks", (prev) => prev.filter((b) => b.id !== id));
  } catch (e) {
    console.error("Failed to remove bookmark:", e);
  }
}

async function refreshBookmarks(folder?: string): Promise<void> {
  try {
    const f = folder ?? null;
    const bookmarks = await ipcGetBookmarks(folder);
    setBookmarkState({ bookmarks, currentFolder: f });
  } catch (e) {
    console.error("Failed to refresh bookmarks:", e);
  }
}

function navigateToFolder(folder: string | null): void {
  refreshBookmarks(folder ?? undefined);
}

export {
  bookmarkState,
  initializeBookmarkStore,
  addBookmarkAction,
  removeBookmarkAction,
  refreshBookmarks,
  navigateToFolder,
};
