import { invoke } from "@tauri-apps/api/core";

// ── Types ────────────────────────────────────────────────────────────

export interface TabInfo {
  id: string;
  title: string;
  url: string;
  isLoading: boolean;
  faviconUrl: string | null;
}

export interface TabsResponse {
  tabs: TabInfo[];
  activeTabId: string | null;
}

// ── Tab commands ─────────────────────────────────────────────────────

export async function createTab(url?: string): Promise<TabInfo> {
  return invoke("create_tab", { url: url ?? null });
}

export async function closeTab(tabId: string): Promise<void> {
  return invoke("close_tab", { tabId });
}

export async function switchTab(tabId: string): Promise<void> {
  return invoke("switch_tab", { tabId });
}

export async function getTabs(): Promise<TabsResponse> {
  return invoke("get_tabs");
}

export async function reorderTabs(tabIds: string[]): Promise<void> {
  return invoke("reorder_tabs", { tabIds });
}

// ── Navigation commands ──────────────────────────────────────────────

export async function navigateTo(tabId: string, input: string): Promise<void> {
  return invoke("navigate_to", { tabId, input });
}

export async function goBack(): Promise<void> {
  return invoke("go_back");
}

export async function goForward(): Promise<void> {
  return invoke("go_forward");
}

export async function reloadPage(): Promise<void> {
  return invoke("reload_page");
}

export async function getCurrentUrl(): Promise<string> {
  return invoke("get_current_url");
}

// ── Privacy commands ────────────────────────────────────────────────

export async function getBlockedCount(tabId: string): Promise<number> {
  return invoke("get_blocked_count", { tabId });
}

export async function toggleShield(tabId: string): Promise<boolean> {
  return invoke("toggle_shield", { tabId });
}

export async function allowHttpAndNavigate(
  tabId: string,
  url: string
): Promise<void> {
  return invoke("allow_http_and_navigate", { tabId, url });
}

export async function toggleSiteShield(domain: string): Promise<boolean> {
  return invoke("toggle_site_shield", { domain });
}

export async function getSiteShieldStatus(domain: string): Promise<boolean> {
  return invoke("get_site_shield_status", { domain });
}

// ── Bookmark types & commands ───────────────────────────────────────

export interface Bookmark {
  id: string;
  url: string;
  title: string;
  folder: string | null;
  faviconData: number[] | null;
  createdAt: string;
}

export async function addBookmark(
  url: string,
  title: string,
  folder?: string
): Promise<Bookmark> {
  return invoke("add_bookmark", { url, title, folder: folder ?? null });
}

export async function removeBookmark(id: string): Promise<void> {
  return invoke("remove_bookmark", { id });
}

export async function getBookmarks(folder?: string): Promise<Bookmark[]> {
  return invoke("get_bookmarks", { folder: folder ?? null });
}

export async function searchBookmarks(query: string): Promise<Bookmark[]> {
  return invoke("search_bookmarks", { query });
}

// ── Settings commands ───────────────────────────────────────────────

export async function getSetting(key: string): Promise<string | null> {
  return invoke("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke("set_setting", { key, value });
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return invoke("get_all_settings");
}

// ── History commands ────────────────────────────────────────────────

export interface HistoryEntry {
  url: string;
  title: string;
  timestamp: number;
}

export async function searchHistory(query: string): Promise<HistoryEntry[]> {
  return invoke("search_history", { query });
}

export async function addHistoryEntry(
  url: string,
  title: string
): Promise<void> {
  return invoke("add_history_entry", { url, title });
}

// ── Layout commands ────────────────────────────────────────────────

export async function setSidebarOpen(open: boolean): Promise<void> {
  return invoke("set_sidebar_open", { open });
}

export async function setSettingsOpen(open: boolean): Promise<void> {
  return invoke("set_settings_open", { open });
}
