import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { LazyStore } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { type LanguagePreference, updateLocale } from "./i18n";

export type Settings = {
  baseUrl: string;
  apiKey: string;
  model: string;
  prompt: string;
  fontFamily: string;
  fontSize: number;
  serverPort: number;
  keepOnTop: boolean;
  monitorClipboard: boolean;
  openaiCompatibleInput: boolean;
  replacements: TextReplacementRule[];
  language: LanguagePreference;
};

export const defaultSettings: Settings = {
  baseUrl: "https://api.openai.com",
  apiKey: "",
  model: "gpt-4o-mini",
  prompt: "你是一个 Galgame 文本翻译助手，请将原文翻译为简洁、流畅的中文对白，保留原有格式与人名。",
  fontFamily: '"Noto Sans CJK SC", "Noto Sans SC", sans-serif',
  fontSize: 18,
  serverPort: 17889,
  keepOnTop: false,
  monitorClipboard: false,
  openaiCompatibleInput: false,
  replacements: [],
  language: "system",
};

export type TextReplacementRule = {
  pattern: string;
  replacement: string;
  flags?: string;
};

const STORAGE_FILE = "settings.json";
const STORAGE_KEY = "translator-settings";
const store = new LazyStore(STORAGE_FILE);

const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_METADATA__" in window || "__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);

function parseSettings(raw: string | null): Settings {
  if (!raw) return { ...defaultSettings };
  try {
    const parsed = JSON.parse(raw) as Partial<Settings>;
    return { ...defaultSettings, ...parsed };
  } catch (error) {
    console.error("Failed to parse settings:", error);
    return { ...defaultSettings };
  }
}

async function loadPersistedSettings(): Promise<Settings> {
  if (!isTauri) {
    return parseSettings(localStorage.getItem(STORAGE_KEY));
  }

  try {
    await store.init();
    const stored = await store.get<Partial<Settings>>(STORAGE_KEY);
    if (stored && Object.keys(stored).length > 0) {
      return { ...defaultSettings, ...stored };
    }
  } catch (error) {
    console.error("Failed to load settings from store, falling back to defaults:", error);
  }

  const legacy = localStorage.getItem(STORAGE_KEY);
  if (legacy) {
    const parsed = parseSettings(legacy);
    await persistSettings(parsed);
    localStorage.removeItem(STORAGE_KEY);
    return parsed;
  }

  return { ...defaultSettings };
}

async function syncTextReplacements(rules: TextReplacementRule[]) {
  if (!isTauri) return;

  try {
    await invoke("set_text_replacements", {
      rules,
    });
  } catch (error) {
    console.error("Failed to sync text replacements to backend:", error);
  }
}

async function persistSettings(val: Settings) {
  if (!isTauri) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(val));
    return;
  }

  try {
    await store.init();
    await store.set(STORAGE_KEY, val);
    await store.save();
  } catch (error) {
    console.error("Failed to persist settings to store:", error);
  }
}

export function useSettingsState() {
  const settings = ref<Settings>({ ...defaultSettings });
  let syncingFromStore = false;
  let unlisten: UnlistenFn | null = null;

  const refreshFromPersistence = async () => {
    syncingFromStore = true;
    settings.value = await loadPersistedSettings();
    await syncTextReplacements(settings.value.replacements);
    updateLocale(settings.value.language);
    setTimeout(() => {
      syncingFromStore = false;
    }, 0);
  };

  const handleStorageEvent = (event: StorageEvent) => {
    if (event.key && event.key !== STORAGE_KEY) return;
    void refreshFromPersistence();
  };

  onMounted(async () => {
    await refreshFromPersistence();

    if (isTauri) {
      try {
        await store.init();
        unlisten = await store.onChange(() => {
          void refreshFromPersistence();
        });
      } catch (error) {
        console.error("Failed to subscribe to store changes:", error);
      }
    } else {
      window.addEventListener("storage", handleStorageEvent);
    }
  });

  onBeforeUnmount(() => {
    if (unlisten) {
      unlisten();
    }
    if (!isTauri) {
      window.removeEventListener("storage", handleStorageEvent);
    }
  });

  watch(
    settings,
    (val) => {
      if (syncingFromStore) return;
      void persistSettings(val);
      void syncTextReplacements(val.replacements);
      updateLocale(val.language);
    },
    { deep: true }
  );

  return settings;
}
