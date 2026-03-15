import "./styles/global.css";
import { onMount, onCleanup, createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { NavigationControls } from "./components/browser/NavigationControls";
import { AddressBar } from "./components/browser/AddressBar";
import { TabBar } from "./components/browser/TabBar";
import { FindBar } from "./components/browser/FindBar";
import { Sidebar } from "./components/sidebar/Sidebar";
import { SettingsPage } from "./components/settings/SettingsPage";
import {
  initializeTabStore,
  tabState,
  createNewTab,
  closeTabAction,
  switchToTab,
  getActiveTab,
} from "./stores/tabStore";
import { initializePrivacyStore } from "./stores/privacyStore";
import { initializeBookmarkStore, addBookmarkAction } from "./stores/bookmarkStore";
import { initializeSettingsStore } from "./stores/settingsStore";
import { toggleSidebar, toggleSettings } from "./stores/sidebarStore";
import { zoomIn, zoomOut, zoomReset } from "./lib/ipc";

function App() {
  const [findBarVisible, setFindBarVisible] = createSignal(false);
  let unlistenShortcut: (() => void) | undefined;

  onMount(() => {
    initializeTabStore();
    initializePrivacyStore();
    initializeBookmarkStore();
    initializeSettingsStore();
  });

  // Listen for keyboard shortcut events forwarded from content webviews
  onMount(async () => {
    unlistenShortcut = await listen<string>("menu-shortcut", (event) => {
      handleShortcutAction(event.payload);
    });
  });

  onCleanup(() => {
    unlistenShortcut?.();
  });

  const handleShortcutAction = (key: string) => {
    switch (key) {
      case "toggle_sidebar":
        toggleSidebar();
        break;
      case "bookmark_page": {
        const activeTab = getActiveTab();
        if (activeTab && activeTab.url && !activeTab.url.startsWith("void://")) {
          addBookmarkAction(activeTab.url, activeTab.title || activeTab.url);
        }
        break;
      }
      case "open_settings":
        toggleSettings();
        break;
      case "new_tab":
        createNewTab();
        break;
      case "close_tab":
        if (tabState.activeTabId) closeTabAction(tabState.activeTabId);
        break;
      case "focus_address_bar": {
        const input = document.querySelector<HTMLInputElement>("[data-address-bar]");
        input?.focus();
        break;
      }
      case "find_in_page":
        setFindBarVisible(true);
        break;
      case "zoom_in":
        zoomIn().catch(console.error);
        break;
      case "zoom_out":
        zoomOut().catch(console.error);
        break;
      case "zoom_reset":
        zoomReset().catch(console.error);
        break;
    }
  };

  // ── Keyboard shortcuts (work when toolbar/main webview has focus) ──
  const handleKeyDown = (e: KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey;

    if (ctrl && e.key === "t") {
      e.preventDefault();
      createNewTab();
      return;
    }

    if (ctrl && e.key === "w") {
      e.preventDefault();
      if (tabState.activeTabId) {
        closeTabAction(tabState.activeTabId);
      }
      return;
    }

    if (ctrl && e.key === "l") {
      e.preventDefault();
      const input = document.querySelector<HTMLInputElement>("[data-address-bar]");
      input?.focus();
      return;
    }

    if (ctrl && e.key === "b") {
      e.preventDefault();
      toggleSidebar();
      return;
    }

    if (ctrl && e.key === "d") {
      e.preventDefault();
      const activeTab = getActiveTab();
      if (activeTab && activeTab.url && !activeTab.url.startsWith("void://")) {
        addBookmarkAction(activeTab.url, activeTab.title || activeTab.url);
      }
      return;
    }

    if (ctrl && e.key === ",") {
      e.preventDefault();
      toggleSettings();
      return;
    }

    if (ctrl && e.key === "f") {
      e.preventDefault();
      setFindBarVisible(true);
      return;
    }

    if (ctrl && (e.key === "=" || e.key === "+")) {
      e.preventDefault();
      zoomIn().catch(console.error);
      return;
    }

    if (ctrl && e.key === "-") {
      e.preventDefault();
      zoomOut().catch(console.error);
      return;
    }

    if (ctrl && e.key === "0") {
      e.preventDefault();
      zoomReset().catch(console.error);
      return;
    }

    // Ctrl+Tab / Ctrl+Shift+Tab — cycle tabs
    if (ctrl && e.key === "Tab") {
      e.preventDefault();
      const tabs = tabState.tabs;
      if (tabs.length < 2) return;
      const currentIdx = tabs.findIndex((t) => t.id === tabState.activeTabId);
      if (currentIdx < 0) return;
      const next = e.shiftKey
        ? (currentIdx - 1 + tabs.length) % tabs.length
        : (currentIdx + 1) % tabs.length;
      switchToTab(tabs[next].id);
      return;
    }

    // Ctrl+1 through Ctrl+9
    if (ctrl && e.key >= "1" && e.key <= "9") {
      e.preventDefault();
      const tabs = tabState.tabs;
      const idx = e.key === "9" ? tabs.length - 1 : parseInt(e.key) - 1;
      if (idx >= 0 && idx < tabs.length) {
        switchToTab(tabs[idx].id);
      }
      return;
    }
  };

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
  });

  return (
    <div class="flex flex-col" style={{ height: "82px" }}>
      <TabBar />
      <div class="h-[46px] bg-neutral-800 border-b border-neutral-700 flex items-center px-2">
        <NavigationControls />
        <AddressBar />
        {/* Sidebar toggle button */}
        <button
          class="w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200 ml-1"
          onClick={() => toggleSidebar()}
          title="Toggle sidebar (Ctrl+B)"
        >
          <svg class="w-4 h-4" viewBox="0 0 16 16" fill="none">
            <path
              d="M2 3h12M2 8h12M2 13h8"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
        {/* Settings button */}
        <button
          class="w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200 ml-1"
          onClick={() => toggleSettings()}
          title="Settings (Ctrl+,)"
        >
          <svg class="w-4 h-4" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.5" />
            <path
              d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.1 3.1l1.4 1.4M11.5 11.5l1.4 1.4M3.1 12.9l1.4-1.4M11.5 4.5l1.4-1.4"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
      <FindBar visible={findBarVisible()} onClose={() => setFindBarVisible(false)} />
      <Sidebar />
      <SettingsPage />
    </div>
  );
}

export default App;
