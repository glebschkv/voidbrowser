import { Show, createSignal } from "solid-js";
import { sidebarOpen, closeSidebar } from "../../stores/sidebarStore";
import { BookmarkPanel } from "./BookmarkPanel";
import { HistoryPanel } from "./HistoryPanel";

type PanelTab = "bookmarks" | "history";

export function Sidebar() {
  const [activePanel, setActivePanel] = createSignal<PanelTab>("bookmarks");

  return (
    <Show when={sidebarOpen()}>
      <div
        class="fixed left-0 bottom-0 w-[300px] bg-neutral-800 border-r border-neutral-700 flex flex-col z-50"
        style={{ top: "82px" }}
      >
        {/* Header */}
        <div class="flex items-center justify-between px-3 py-2 border-b border-neutral-700">
          <div class="flex gap-1">
            <button
              class={`px-3 py-1 rounded text-xs font-medium ${
                activePanel() === "bookmarks"
                  ? "bg-neutral-600 text-neutral-100"
                  : "text-neutral-400 hover:text-neutral-200"
              }`}
              onClick={() => setActivePanel("bookmarks")}
            >
              Bookmarks
            </button>
            <button
              class={`px-3 py-1 rounded text-xs font-medium ${
                activePanel() === "history"
                  ? "bg-neutral-600 text-neutral-100"
                  : "text-neutral-400 hover:text-neutral-200"
              }`}
              onClick={() => setActivePanel("history")}
            >
              History
            </button>
          </div>
          <button
            class="w-6 h-6 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200"
            onClick={() => closeSidebar()}
            title="Close sidebar"
          >
            <svg class="w-4 h-4" viewBox="0 0 16 16" fill="none">
              <path
                d="M4 4l8 8M12 4l-8 8"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>

        {/* Panel content */}
        <div class="flex-1 overflow-y-auto">
          <Show when={activePanel() === "bookmarks"}>
            <BookmarkPanel />
          </Show>
          <Show when={activePanel() === "history"}>
            <HistoryPanel />
          </Show>
        </div>
      </div>
    </Show>
  );
}
