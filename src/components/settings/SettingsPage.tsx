import { Show, onMount } from "solid-js";
import { settingsOpen, setSettingsOpen } from "../../stores/sidebarStore";
import {
  settingsState,
  initializeSettingsStore,
  updateSetting,
  getSettingValue,
} from "../../stores/settingsStore";

function SelectSetting(props: {
  label: string;
  settingKey: string;
  options: { value: string; label: string }[];
}) {
  return (
    <div class="flex items-center justify-between py-2">
      <span class="text-xs text-neutral-300">{props.label}</span>
      <select
        class="bg-neutral-700 text-xs text-neutral-200 px-2 py-1 rounded outline-none focus:ring-1 focus:ring-accent"
        value={getSettingValue(props.settingKey)}
        onChange={(e) => updateSetting(props.settingKey, e.currentTarget.value)}
      >
        {props.options.map((opt) => (
          <option value={opt.value}>{opt.label}</option>
        ))}
      </select>
    </div>
  );
}

function ToggleSetting(props: { label: string; settingKey: string }) {
  const isOn = () => getSettingValue(props.settingKey) === "true";

  return (
    <div class="flex items-center justify-between py-2">
      <span class="text-xs text-neutral-300">{props.label}</span>
      <button
        class={`w-9 h-5 rounded-full relative transition-colors ${
          isOn() ? "bg-accent" : "bg-neutral-600"
        }`}
        onClick={() => updateSetting(props.settingKey, isOn() ? "false" : "true")}
      >
        <div
          class={`w-3.5 h-3.5 bg-white rounded-full absolute top-0.5 transition-transform ${
            isOn() ? "translate-x-[18px]" : "translate-x-0.5"
          }`}
        />
      </button>
    </div>
  );
}

function SectionHeader(props: { title: string }) {
  return (
    <h3 class="text-[10px] uppercase tracking-wider text-neutral-500 font-semibold mt-4 mb-1">
      {props.title}
    </h3>
  );
}

export function SettingsPage() {
  onMount(() => {
    initializeSettingsStore();
  });

  return (
    <Show when={settingsOpen()}>
      <div
        class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
        style={{ top: "82px" }}
        onClick={(e) => {
          if (e.target === e.currentTarget) setSettingsOpen(false);
        }}
      >
        <div class="w-[420px] max-h-[80vh] bg-neutral-800 rounded-lg border border-neutral-700 shadow-xl overflow-hidden flex flex-col">
          {/* Header */}
          <div class="flex items-center justify-between px-4 py-3 border-b border-neutral-700">
            <h2 class="text-sm font-medium text-neutral-100">Settings</h2>
            <button
              class="w-6 h-6 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200"
              onClick={() => setSettingsOpen(false)}
            >
              <svg class="w-4 h-4" viewBox="0 0 16 16" fill="none">
                <path
                  d="M4 4l8 8M12 4l-8 8"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linecap="round"
                />
              </svg>
            </button>
          </div>

          {/* Content */}
          <div class="flex-1 overflow-y-auto px-4 pb-4">
            <SectionHeader title="Search" />
            <SelectSetting
              label="Default search engine"
              settingKey="search_engine"
              options={[
                { value: "duckduckgo", label: "DuckDuckGo" },
                { value: "brave", label: "Brave Search" },
                { value: "startpage", label: "Startpage" },
                { value: "google", label: "Google" },
              ]}
            />

            <SectionHeader title="Privacy" />
            <ToggleSetting label="Shield (ad & tracker blocking)" settingKey="shield_enabled" />
            <ToggleSetting label="Fingerprint resistance" settingKey="fingerprint_resistance" />
            <ToggleSetting label="HTTPS-only mode" settingKey="https_only" />
            <SelectSetting
              label="Third-party cookies"
              settingKey="third_party_cookies"
              options={[
                { value: "block", label: "Block all" },
                { value: "allow", label: "Allow" },
              ]}
            />
            <SelectSetting
              label="First-party cookies"
              settingKey="first_party_cookies"
              options={[
                { value: "session_only", label: "Session only" },
                { value: "allow", label: "Allow" },
              ]}
            />

            <SectionHeader title="Appearance" />
            <SelectSetting
              label="Theme"
              settingKey="theme"
              options={[{ value: "dark", label: "Dark" }]}
            />
            <SelectSetting
              label="Font size"
              settingKey="font_size"
              options={[
                { value: "small", label: "Small" },
                { value: "medium", label: "Medium" },
                { value: "large", label: "Large" },
              ]}
            />

            <SectionHeader title="Behavior" />
            <ToggleSetting
              label="Restore tabs on start"
              settingKey="restore_tabs_on_start"
            />
            <SelectSetting
              label="History mode"
              settingKey="history_mode"
              options={[
                { value: "session_only", label: "Session only" },
              ]}
            />
          </div>
        </div>
      </div>
    </Show>
  );
}
