import { createStore } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { toggleShield, toggleSiteShield, getSiteShieldStatus } from "../lib/ipc";

interface PrivacyState {
  blockedCounts: Record<string, number>;
  disabledSites: Record<string, boolean>;
  initialized: boolean;
}

const [privacyState, setPrivacyState] = createStore<PrivacyState>({
  blockedCounts: {},
  disabledSites: {},
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

  await listen<{ domain: string; enabled: boolean }>(
    "site-shield-toggled",
    (event) => {
      setPrivacyState(
        "disabledSites",
        event.payload.domain,
        !event.payload.enabled
      );
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

function isSiteShieldDisabled(domain: string): boolean {
  return privacyState.disabledSites[domain] ?? false;
}

async function toggleShieldAction(tabId: string): Promise<boolean> {
  try {
    return await toggleShield(tabId);
  } catch (e) {
    console.error("Failed to toggle shield:", e);
    return true;
  }
}

async function toggleSiteShieldAction(domain: string): Promise<boolean> {
  try {
    return await toggleSiteShield(domain);
  } catch (e) {
    console.error("Failed to toggle site shield:", e);
    return true;
  }
}

async function fetchSiteShieldStatus(domain: string): Promise<boolean> {
  try {
    return await getSiteShieldStatus(domain);
  } catch (e) {
    console.error("Failed to get site shield status:", e);
    return true;
  }
}

export {
  privacyState,
  initializePrivacyStore,
  getBlockedCountForTab,
  isSiteShieldDisabled,
  toggleShieldAction,
  toggleSiteShieldAction,
  fetchSiteShieldStatus,
};
