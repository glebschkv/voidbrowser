import { createSignal, onMount, onCleanup, Show } from "solid-js";
import { tabState } from "../../stores/tabStore";
import {
  getBlockedCountForTab,
  isSiteShieldDisabled,
  toggleSiteShieldAction,
} from "../../stores/privacyStore";

interface SitePermissionsProps {
  onClose: () => void;
}

function extractDomain(url: string): string {
  try {
    const parsed = new URL(url);
    return parsed.hostname;
  } catch {
    return "";
  }
}

export function SitePermissions(props: SitePermissionsProps) {
  let dropdownRef: HTMLDivElement | undefined;
  const [shieldEnabled, setShieldEnabled] = createSignal(true);

  const activeTab = () => {
    if (!tabState.activeTabId) return null;
    return tabState.tabs.find((t) => t.id === tabState.activeTabId) ?? null;
  };

  const domain = () => {
    const tab = activeTab();
    if (!tab || !tab.url) return "";
    return extractDomain(tab.url);
  };

  const blockedCount = () => getBlockedCountForTab(tabState.activeTabId);

  // Sync shield state from store
  const updateShieldState = () => {
    const d = domain();
    if (d) {
      setShieldEnabled(!isSiteShieldDisabled(d));
    }
  };

  onMount(() => {
    updateShieldState();

    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
        props.onClose();
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        props.onClose();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);

    onCleanup(() => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    });
  });

  const handleToggle = async () => {
    const d = domain();
    if (!d) return;
    const enabled = await toggleSiteShieldAction(d);
    setShieldEnabled(enabled);
  };

  return (
    <div
      ref={dropdownRef}
      class="absolute top-full right-0 mt-1 w-72 bg-neutral-800 border border-neutral-600 rounded-lg shadow-lg z-50 overflow-hidden"
    >
      <div class="px-4 py-3 border-b border-neutral-700">
        <div class="flex items-center gap-2">
          <div
            class={`w-2 h-2 rounded-full ${
              shieldEnabled() ? "bg-green-400" : "bg-neutral-500"
            }`}
          />
          <span class="text-sm font-medium text-neutral-200 truncate">
            {domain() || "No site"}
          </span>
        </div>
      </div>

      <Show when={domain()}>
        <div class="px-4 py-3">
          <button
            class="w-full flex items-center justify-between px-3 py-2 rounded-md hover:bg-neutral-700 transition-colors"
            onClick={handleToggle}
          >
            <span class="text-sm text-neutral-200">Protection</span>
            <div
              class={`w-9 h-5 rounded-full relative transition-colors ${
                shieldEnabled() ? "bg-green-500" : "bg-neutral-600"
              }`}
            >
              <div
                class={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                  shieldEnabled() ? "left-4.5 translate-x-0" : "left-0.5"
                }`}
                style={{
                  left: shieldEnabled() ? "18px" : "2px",
                }}
              />
            </div>
          </button>

          <Show when={shieldEnabled()}>
            <div class="mt-2 px-3 py-2 bg-neutral-750 rounded-md">
              <div class="flex items-center justify-between">
                <span class="text-xs text-neutral-400">Trackers blocked</span>
                <span class="text-xs font-medium text-neutral-200">
                  {blockedCount()}
                </span>
              </div>
            </div>
          </Show>

          <Show when={!shieldEnabled()}>
            <p class="mt-2 px-3 text-xs text-neutral-500">
              Ad blocking and fingerprint resistance are disabled for this site.
            </p>
          </Show>
        </div>
      </Show>

      <div class="px-4 py-2 border-t border-neutral-700">
        <p class="text-[10px] text-neutral-600">
          Resets when browser closes
        </p>
      </div>
    </div>
  );
}
