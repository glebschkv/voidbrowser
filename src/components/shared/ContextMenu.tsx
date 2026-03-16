import { For, onCleanup, onMount } from "solid-js";

export interface ContextMenuItem {
  label: string;
  action: () => void;
  separator?: boolean;
  disabled?: boolean;
}

interface ContextMenuProps {
  items: ContextMenuItem[];
  x: number;
  y: number;
  onClose: () => void;
}

export function ContextMenu(props: ContextMenuProps) {
  let menuRef: HTMLDivElement | undefined;

  const handleClickOutside = (e: MouseEvent) => {
    if (menuRef && !menuRef.contains(e.target as Node)) {
      props.onClose();
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      props.onClose();
    }
  };

  onMount(() => {
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener("mousedown", handleClickOutside);
    document.removeEventListener("keydown", handleKeyDown);
  });

  return (
    <div
      ref={menuRef}
      class="fixed z-50 min-w-[180px] bg-neutral-800 border border-neutral-600 rounded-md shadow-lg py-1"
      style={{ left: `${props.x}px`, top: `${props.y}px` }}
    >
      <For each={props.items}>
        {(item) => (
          <>
            {item.separator && (
              <div class="h-px bg-neutral-600 my-1" />
            )}
            <button
              class="w-full text-left px-3 py-1.5 text-sm text-neutral-200 hover:bg-neutral-700 disabled:opacity-40 disabled:pointer-events-none"
              onClick={() => {
                item.action();
                props.onClose();
              }}
              disabled={item.disabled}
            >
              {item.label}
            </button>
          </>
        )}
      </For>
    </div>
  );
}
