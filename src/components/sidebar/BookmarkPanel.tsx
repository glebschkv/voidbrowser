import { Show, For, onMount } from "solid-js";
import {
  bookmarkState,
  initializeBookmarkStore,
  addBookmarkAction,
  removeBookmarkAction,
  navigateToFolder,
} from "../../stores/bookmarkStore";
import { tabState } from "../../stores/tabStore";
import { navigateTo } from "../../lib/ipc";

export function BookmarkPanel() {
  onMount(() => {
    initializeBookmarkStore();
  });

  const handleAddBookmark = () => {
    const activeTab = tabState.tabs.find((t) => t.id === tabState.activeTabId);
    if (activeTab && activeTab.url && !activeTab.url.startsWith("void://")) {
      addBookmarkAction(activeTab.url, activeTab.title || activeTab.url);
    }
  };

  const handleClick = (url: string) => {
    if (tabState.activeTabId) {
      navigateTo(tabState.activeTabId, url).catch(console.error);
    }
  };

  const handleRemove = (e: MouseEvent, id: string) => {
    e.stopPropagation();
    removeBookmarkAction(id);
  };

  return (
    <div class="flex flex-col h-full">
      {/* Folder breadcrumb */}
      <Show when={bookmarkState.currentFolder}>
        <div class="px-3 py-2 border-b border-neutral-700 flex items-center gap-2">
          <button
            class="text-xs text-accent hover:underline"
            onClick={() => navigateToFolder(null)}
          >
            Root
          </button>
          <span class="text-neutral-500 text-xs">/</span>
          <span class="text-xs text-neutral-300">
            {bookmarkState.currentFolder}
          </span>
        </div>
      </Show>

      {/* Add bookmark button */}
      <div class="px-3 py-2 border-b border-neutral-700">
        <button
          class="w-full px-3 py-1.5 bg-accent/20 text-accent text-xs rounded hover:bg-accent/30 font-medium"
          onClick={handleAddBookmark}
        >
          + Bookmark current page
        </button>
      </div>

      {/* Bookmark list */}
      <div class="flex-1 overflow-y-auto">
        <Show
          when={bookmarkState.bookmarks.length > 0}
          fallback={
            <div class="px-3 py-8 text-center text-neutral-500 text-xs">
              No bookmarks yet
            </div>
          }
        >
          <For each={bookmarkState.bookmarks}>
            {(bookmark) => (
              <div
                class="flex items-center gap-2 px-3 py-2 hover:bg-neutral-700 cursor-pointer group"
                onClick={() => handleClick(bookmark.url)}
              >
                <svg
                  class="w-4 h-4 text-accent flex-shrink-0"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                >
                  <path d="M8 1.5l1.76 3.57 3.94.57-2.85 2.78.67 3.93L8 10.42l-3.52 1.93.67-3.93L2.3 5.64l3.94-.57L8 1.5z" />
                </svg>
                <div class="flex-1 min-w-0">
                  <div class="text-xs text-neutral-200 truncate">
                    {bookmark.title}
                  </div>
                  <div class="text-[10px] text-neutral-500 truncate">
                    {bookmark.url}
                  </div>
                </div>
                <button
                  class="w-5 h-5 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-500 hover:text-neutral-200 opacity-0 group-hover:opacity-100"
                  onClick={(e) => handleRemove(e, bookmark.id)}
                  title="Remove bookmark"
                >
                  <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none">
                    <path
                      d="M4 4l8 8M12 4l-8 8"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                    />
                  </svg>
                </button>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}
