import { onBeforeUnmount, onMounted, ref, watch } from "vue";

export type Settings = {
  baseUrl: string;
  apiKey: string;
  model: string;
  prompt: string;
  fontFamily: string;
  fontSize: number;
  serverPort: number;
  keepOnTop: boolean;
};

export const defaultSettings: Settings = {
  baseUrl: "https://api.openai.com",
  apiKey: "",
  model: "gpt-4o-mini",
  prompt: "你是一个 Galgame 文本翻译助手，请将原文翻译为简洁、流畅的中文对白，保留原有格式与人名。",
  fontFamily: "\"LXGW WenKai\", \"Noto Sans SC\", \"Space Grotesk\", sans-serif",
  fontSize: 18,
  serverPort: 17889,
  keepOnTop: true
};

const STORAGE_KEY = "translator-settings";

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

export function loadSettings(): Settings {
  return parseSettings(localStorage.getItem(STORAGE_KEY));
}

export function useSettingsState() {
  const settings = ref<Settings>(loadSettings());

  const handleStorage = (event: StorageEvent) => {
    if (event.key && event.key !== STORAGE_KEY) return;
    settings.value = parseSettings(event.newValue ?? localStorage.getItem(STORAGE_KEY));
  };

  onMounted(() => {
    window.addEventListener("storage", handleStorage);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("storage", handleStorage);
  });

  watch(
    settings,
    (val) => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(val));
    },
    { deep: true }
  );

  return settings;
}
