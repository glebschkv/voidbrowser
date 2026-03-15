import { createSignal, Show } from "solid-js";
import { tabState } from "../../stores/tabStore";
import { getBlockedCountForTab, isSiteShieldDisabled } from "../../stores/privacyStore";
import { SitePermissions } from "./SitePermissions";

function extractDomain(url: string): string {
  try {
    const parsed = new URL(url);
    return parsed.hostname;
  } catch {
    return "";
  }
}

export function ShieldIcon() {
  const [showDropdown, setShowDropdown] = createSignal(false);

  const blockedCount = () => getBlockedCountForTab(tabState.activeTabId);

  const activeTab = () => {
    if (!tabState.activeTabId) return null;
    return tabState.tabs.find((t) => t.id === tabState.activeTabId) ?? null;
  };

  const domain = () => {
    const tab = activeTab();
    if (!tab || !tab.url) return "";
    return extractDomain(tab.url);
  };

  const isDisabled = () => {
    const d = domain();
    if (!d) return false;
    return isSiteShieldDisabled(d);
  };

  const shieldColor = () => (isDisabled() ? "text-neutral-500" : "text-green-400");

  const handleClick = () => {
    setShowDropdown((prev) => !prev);
  };

  return (
    <div class="relative">
      <button
        class={`relative flex items-center justify-center w-8 h-8 rounded hover:bg-neutral-600 flex-shrink-0 ${shieldColor()}`}
        onClick={handleClick}
        title={
          isDisabled()
            ? "Shield disabled for this site"
            : `Shield active — ${blockedCount()} blocked`
        }
      >
        <svg
          class="w-4 h-4"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        >
          <path
            d="M8 1.5L2.5 4v4c0 3.5 2.5 5.5 5.5 6.5 3-1 5.5-3 5.5-6.5V4L8 1.5z"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          {!isDisabled() && (
            <path
              d="M5.5 8.5l1.5 1.5 3.5-3.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          )}
        </svg>
        {blockedCount() > 0 && !isDisabled() && (
          <span class="absolute -top-1 -right-1 min-w-[16px] h-4 px-1 bg-indigo-500 text-white text-[10px] font-bold rounded-full flex items-center justify-center leading-none">
            {blockedCount() > 99 ? "99+" : blockedCount()}
          </span>
        )}
      </button>
      <Show when={showDropdown()}>
        <SitePermissions onClose={() => setShowDropdown(false)} />
      </Show>
    </div>
  );
}
