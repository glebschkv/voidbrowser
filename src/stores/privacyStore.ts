import { createStore } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { getBlockedCount, toggleShield } from "../lib/ipc";

interface PrivacyState {
  blockedCounts: Record<string, number>;
  initialized: boolean;
}

const [privacyState, setPrivacyState] = createStore<PrivacyState>({
  blockedCounts: {},
  initialized: false,
});

let listenersSetup = false;

async function setupListeners(): Promise<void> {
  if (listenersSetup) return;
  listenersSetup = true;

  await listen<{ tabId: string; count: number }>(
    "blocked-count-updated",
    (event) => {
      setPrivacyState("blockedCounts", event.payload.tabId, event.payload.count);
    }
  );
}

async function initializePrivacyStore(): Promise<void> {
  if (privacyState.initialized) return;
  await setupListeners();
  setPrivacyState("initialized", true);
}

function getBlockedCountForTab(tabId: string | null): number {
  if (!tabId) return 0;
  return privacyState.blockedCounts[tabId] ?? 0;
}

async function toggleShieldAction(tabId: string): Promise<boolean> {
  try {
    return await toggleShield(tabId);
  } catch (e) {
    console.error("Failed to toggle shield:", e);
    return true;
  }
}

export {
  privacyState,
  initializePrivacyStore,
  getBlockedCountForTab,
  toggleShieldAction,
};
