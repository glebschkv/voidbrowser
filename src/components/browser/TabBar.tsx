import { For } from "solid-js";
import { Tab } from "./Tab";
import {
  tabState,
  createNewTab,
  closeTabAction,
  switchToTab,
} from "../../stores/tabStore";
import { createTab as ipcCreateTab, closeTab as ipcCloseTab } from "../../lib/ipc";

export function TabBar() {
  const handleCloseOthers = async (keepId: string) => {
    const toClose = tabState.tabs.filter((t) => t.id !== keepId);
    for (const tab of toClose) {
      await ipcCloseTab(tab.id).catch(console.error);
    }
  };

  const handleCloseRight = async (fromId: string) => {
    const idx = tabState.tabs.findIndex((t) => t.id === fromId);
    if (idx < 0) return;
    const toClose = tabState.tabs.slice(idx + 1);
    for (const tab of toClose) {
      await ipcCloseTab(tab.id).catch(console.error);
    }
  };

  const handleDuplicate = (tabId: string) => {
    const tab = tabState.tabs.find((t) => t.id === tabId);
    if (tab) {
      ipcCreateTab(tab.url).catch(console.error);
    }
  };

  const handleDoubleClick = (e: MouseEvent) => {
    // Only on the tab bar background, not on a tab
    if ((e.target as HTMLElement).closest("[data-tab]")) return;
    createNewTab();
  };

  return (
    <div
      class="flex items-center h-[36px] bg-neutral-800 border-b border-neutral-700 overflow-x-auto"
      style={{ "scrollbar-width": "none" }}
      onDblClick={handleDoubleClick}
    >
      <div class="flex items-end h-full flex-1 min-w-0">
        <For each={tabState.tabs}>
          {(tab) => (
            <div data-tab>
              <Tab
                id={tab.id}
                title={tab.title}
                url={tab.url}
                isActive={tab.id === tabState.activeTabId}
                faviconUrl={tab.faviconUrl}
                onClose={closeTabAction}
                onSelect={switchToTab}
                onCloseOthers={handleCloseOthers}
                onCloseRight={handleCloseRight}
                onDuplicate={handleDuplicate}
              />
            </div>
          )}
        </For>
      </div>

      {/* New tab button */}
      <button
        class="flex-shrink-0 w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-100 transition-colors mx-1"
        onClick={() => createNewTab()}
        title="New tab (Ctrl+T)"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path
            d="M7 1V13M1 7H13"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
  );
}
