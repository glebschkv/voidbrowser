import { createStore } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import {
  createTab as ipcCreateTab,
  closeTab as ipcCloseTab,
  switchTab as ipcSwitchTab,
  getTabs as ipcGetTabs,
  type TabInfo,
} from "../lib/ipc";

interface TabState {
  tabs: TabInfo[];
  activeTabId: string | null;
  initialized: boolean;
}

const [tabState, setTabState] = createStore<TabState>({
  tabs: [],
  activeTabId: null,
  initialized: false,
});

// ── Event listeners ──────────────────────────────────────────────────

let listenersSetup = false;

async function setupListeners(): Promise<void> {
  if (listenersSetup) return;
  listenersSetup = true;

  await listen<TabInfo>("tab-created", (event) => {
    setTabState("tabs", (tabs) => [...tabs, event.payload]);
  });

  await listen<string>("tab-closed", (event) => {
    const closedId = event.payload;
    setTabState("tabs", (tabs) => tabs.filter((t) => t.id !== closedId));
  });

  await listen<TabInfo>("tab-updated", (event) => {
    const updated = event.payload;
    setTabState("tabs", (t) => t.id === updated.id, updated);
  });

  await listen<string>("active-tab-changed", (event) => {
    setTabState("activeTabId", event.payload);
  });
}

// ── Initialize ───────────────────────────────────────────────────────

async function initializeTabStore(): Promise<void> {
  if (tabState.initialized) return;

  await setupListeners();

  try {
    const response = await ipcGetTabs();
    setTabState({
      tabs: response.tabs,
      activeTabId: response.activeTabId,
      initialized: true,
    });
  } catch (e) {
    console.error("Failed to initialize tab store:", e);
    setTabState("initialized", true);
  }
}

// ── Actions ──────────────────────────────────────────────────────────

async function createNewTab(url?: string): Promise<void> {
  try {
    await ipcCreateTab(url);
  } catch (e) {
    console.error("Failed to create tab:", e);
  }
}

async function closeTabAction(tabId: string): Promise<void> {
  try {
    await ipcCloseTab(tabId);
  } catch (e) {
    console.error("Failed to close tab:", e);
  }
}

async function switchToTab(tabId: string): Promise<void> {
  try {
    await ipcSwitchTab(tabId);
  } catch (e) {
    console.error("Failed to switch tab:", e);
  }
}

function getActiveTab(): TabInfo | undefined {
  return tabState.tabs.find((t) => t.id === tabState.activeTabId);
}

// ── Exports ──────────────────────────────────────────────────────────

export {
  tabState,
  initializeTabStore,
  createNewTab,
  closeTabAction,
  switchToTab,
  getActiveTab,
};
