import { createSignal, Show, createEffect } from "solid-js";
import { ContextMenu, type ContextMenuItem } from "../shared/ContextMenu";

interface TabProps {
  id: string;
  title: string;
  url: string;
  isActive: boolean;
  isLoading: boolean;
  faviconUrl: string | null;
  onClose: (id: string) => void;
  onSelect: (id: string) => void;
  onCloseOthers: (id: string) => void;
  onCloseRight: (id: string) => void;
  onDuplicate: (id: string) => void;
}

export function Tab(props: TabProps) {
  const [contextMenu, setContextMenu] = createSignal<{
    x: number;
    y: number;
  } | null>(null);
  const [faviconError, setFaviconError] = createSignal(false);

  // Reset error state when favicon URL changes
  createEffect(() => {
    props.faviconUrl;
    setFaviconError(false);
  });

  const handleMouseDown = (e: MouseEvent) => {
    // Middle-click to close
    if (e.button === 1) {
      e.preventDefault();
      props.onClose(props.id);
    }
  };

  const handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const contextMenuItems = (): ContextMenuItem[] => [
    { label: "Close tab", action: () => props.onClose(props.id) },
    {
      label: "Close other tabs",
      action: () => props.onCloseOthers(props.id),
    },
    {
      label: "Close tabs to the right",
      action: () => props.onCloseRight(props.id),
      separator: true,
    },
    {
      label: "Duplicate tab",
      action: () => props.onDuplicate(props.id),
      separator: true,
    },
  ];

  return (
    <>
      <div
        class={`group relative flex items-center gap-1.5 h-[32px] min-w-[100px] max-w-[200px] px-3 text-xs cursor-pointer select-none shrink-0 border-r border-neutral-700 ${
          props.isActive
            ? "bg-neutral-700 text-neutral-100"
            : "bg-neutral-800 text-neutral-400 hover:bg-neutral-750 hover:text-neutral-300"
        }`}
        onClick={() => props.onSelect(props.id)}
        onMouseDown={handleMouseDown}
        onContextMenu={handleContextMenu}
      >
        {/* Active tab indicator */}
        <Show when={props.isActive}>
          <div class="absolute bottom-0 left-0 right-0 h-[2px] bg-accent" />
        </Show>

        {/* Favicon / Loading spinner */}
        <Show
          when={!props.isLoading}
          fallback={
            <svg
              class="w-3.5 h-3.5 flex-shrink-0 text-accent animate-spin"
              viewBox="0 0 16 16"
              fill="none"
            >
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" opacity="0.3" />
              <path d="M14 8A6 6 0 0 0 8 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          }
        >
          <Show
            when={props.faviconUrl && !faviconError()}
            fallback={
              <svg
                class="w-3.5 h-3.5 flex-shrink-0 text-neutral-500"
                viewBox="0 0 16 16"
                fill="none"
              >
                <circle
                  cx="8"
                  cy="8"
                  r="6"
                  stroke="currentColor"
                  stroke-width="1.5"
                />
              </svg>
            }
          >
            <img
              src={props.faviconUrl!}
              class="w-3.5 h-3.5 flex-shrink-0"
              alt=""
              onError={() => setFaviconError(true)}
            />
          </Show>
        </Show>

        {/* Title */}
        <span class="flex-1 truncate">{props.title || "New Tab"}</span>

        {/* Close button */}
        <button
          class="w-4 h-4 flex items-center justify-center rounded-sm opacity-0 group-hover:opacity-100 hover:bg-neutral-600 text-neutral-400 hover:text-neutral-100 transition-opacity flex-shrink-0"
          onClick={(e) => {
            e.stopPropagation();
            props.onClose(props.id);
          }}
        >
          <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
            <path
              d="M1 1L7 7M7 1L1 7"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>

      {/* Context menu */}
      <Show when={contextMenu()}>
        <ContextMenu
          items={contextMenuItems()}
          x={contextMenu()!.x}
          y={contextMenu()!.y}
          onClose={() => setContextMenu(null)}
        />
      </Show>
    </>
  );
}
