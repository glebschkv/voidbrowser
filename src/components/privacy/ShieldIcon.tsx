import { createSignal } from "solid-js";
import { tabState } from "../../stores/tabStore";
import { getBlockedCountForTab, toggleShieldAction } from "../../stores/privacyStore";

export function ShieldIcon() {
  const [isDisabled, setIsDisabled] = createSignal(false);

  const blockedCount = () => getBlockedCountForTab(tabState.activeTabId);

  const handleClick = async () => {
    if (!tabState.activeTabId) return;
    const enabled = await toggleShieldAction(tabState.activeTabId);
    setIsDisabled(!enabled);
  };

  const shieldColor = () => (isDisabled() ? "text-neutral-500" : "text-green-400");

  return (
    <button
      class={`relative flex items-center justify-center w-8 h-8 rounded hover:bg-neutral-600 flex-shrink-0 ${shieldColor()}`}
      onClick={handleClick}
      title={
        isDisabled()
          ? "Shield disabled for this tab"
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
  );
}
