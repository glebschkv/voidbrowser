import { createSignal, For, Show } from "solid-js";
import { searchHistory, type HistoryEntry } from "../../lib/ipc";
import { navigateTo } from "../../lib/ipc";
import { tabState } from "../../stores/tabStore";

export function HistoryPanel() {
  const [entries, setEntries] = createSignal<HistoryEntry[]>([]);
  const [query, setQuery] = createSignal("");
  const [loaded, setLoaded] = createSignal(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const doSearch = (q: string) => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      try {
        const results = await searchHistory(q);
        setEntries(results);
        setLoaded(true);
      } catch {
        setEntries([]);
        setLoaded(true);
      }
    }, 150);
  };

  // Load initial entries
  doSearch("");

  const handleInput = (e: InputEvent) => {
    const value = (e.target as HTMLInputElement).value;
    setQuery(value);
    doSearch(value);
  };

  const handleClick = (url: string) => {
    if (tabState.activeTabId) {
      navigateTo(tabState.activeTabId, url).catch(console.error);
    }
  };

  return (
    <div class="flex flex-col h-full">
      {/* Search */}
      <div class="px-3 py-2 border-b border-neutral-700">
        <input
          type="text"
          class="w-full px-2 py-1.5 bg-neutral-700 rounded text-xs text-neutral-200 placeholder-neutral-500 outline-none focus:ring-1 focus:ring-accent"
          placeholder="Search history..."
          value={query()}
          onInput={handleInput}
        />
      </div>

      {/* Info note */}
      <div class="px-3 py-1.5 text-[10px] text-neutral-500 border-b border-neutral-700">
        Session history — cleared when browser closes
      </div>

      {/* Entry list */}
      <div class="flex-1 overflow-y-auto">
        <Show
          when={entries().length > 0}
          fallback={
            <Show when={loaded()}>
              <div class="px-3 py-8 text-center text-neutral-500 text-xs">
                {query() ? "No matching entries" : "No history yet"}
              </div>
            </Show>
          }
        >
          <For each={entries()}>
            {(entry) => (
              <div
                class="flex items-center gap-2 px-3 py-2 hover:bg-neutral-700 cursor-pointer"
                onClick={() => handleClick(entry.url)}
              >
                <svg
                  class="w-4 h-4 text-neutral-500 flex-shrink-0"
                  viewBox="0 0 16 16"
                  fill="none"
                >
                  <circle
                    cx="8"
                    cy="8"
                    r="6"
                    stroke="currentColor"
                    stroke-width="1.5"
                  />
                  <path
                    d="M8 4.5V8l2.5 1.5"
                    stroke="currentColor"
                    stroke-width="1.5"
                    stroke-linecap="round"
                  />
                </svg>
                <div class="flex-1 min-w-0">
                  <div class="text-xs text-neutral-200 truncate">
                    {entry.title || entry.url}
                  </div>
                  <div class="text-[10px] text-neutral-500 truncate">
                    {entry.url}
                  </div>
                </div>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}
