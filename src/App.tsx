import "./styles/global.css";
import { onMount, onCleanup } from "solid-js";
import { NavigationControls } from "./components/browser/NavigationControls";
import { AddressBar } from "./components/browser/AddressBar";
import { TabBar } from "./components/browser/TabBar";
import {
  initializeTabStore,
  tabState,
  createNewTab,
  closeTabAction,
  switchToTab,
} from "./stores/tabStore";

function App() {
  onMount(() => {
    initializeTabStore();
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
    </div>
  );
}

export default App;
