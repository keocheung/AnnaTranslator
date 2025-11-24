import { createI18n } from "vue-i18n";
import en from "./locales/en";
import zhCN from "./locales/zh-CN";

export type LanguagePreference = "system" | "en" | "zh-CN";

const messages = {
  en,
  "zh-CN": zhCN,
};

function detectSystemLocale(): keyof typeof messages {
  if (typeof navigator === "undefined") return "en";
  const lang = navigator.language?.toLowerCase() ?? "";
  if (lang.startsWith("zh")) return "zh-CN";
  return "en";
}

export function resolveLocale(preference: LanguagePreference): keyof typeof messages {
  if (preference === "system") {
    return detectSystemLocale();
  }
  if (preference in messages) {
    return preference as keyof typeof messages;
  }
  return "en";
}

export const i18n = createI18n({
  legacy: false,
  locale: resolveLocale("system"),
  fallbackLocale: "en",
  messages,
});

export function updateLocale(preference: LanguagePreference) {
  i18n.global.locale.value = resolveLocale(preference);
}
