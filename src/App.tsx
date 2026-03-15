import "./styles/global.css";
import { onMount, onCleanup } from "solid-js";
import { NavigationControls } from "./components/browser/NavigationControls";
import { AddressBar } from "./components/browser/AddressBar";
import { TabBar } from "./components/browser/TabBar";
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

function App() {
  onMount(() => {
    initializeTabStore();
    initializePrivacyStore();
    initializeBookmarkStore();
    initializeSettingsStore();
  });

  // ── Keyboard shortcuts ─────────────────────────────────────────────
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

    // Ctrl+B — toggle bookmark sidebar
    if (ctrl && e.key === "b") {
      e.preventDefault();
      toggleSidebar();
      return;
    }

    // Ctrl+D — bookmark current page
    if (ctrl && e.key === "d") {
      e.preventDefault();
      const activeTab = getActiveTab();
      if (activeTab && activeTab.url && !activeTab.url.startsWith("void://")) {
        addBookmarkAction(activeTab.url, activeTab.title || activeTab.url);
      }
      return;
    }

    // Ctrl+, — open settings
    if (ctrl && e.key === ",") {
      e.preventDefault();
      toggleSettings();
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
      </div>
      <Sidebar />
      <SettingsPage />
    </div>
  );
}

export default App;
