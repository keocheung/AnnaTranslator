import { createApp } from "vue";
import "./style.css";
import { i18n } from "./i18n";

const isSettingsWindow = window.location.hash === "#settings";
const isHistoryWindow = window.location.hash === "#history";

async function bootstrap() {
  if (isSettingsWindow) {
    const module = await import("./SettingsWindow.vue");
    createApp(module.default).use(i18n).mount("#app");
    return;
  }

  if (isHistoryWindow) {
    const module = await import("./HistoryWindow.vue");
    createApp(module.default).use(i18n).mount("#app");
    return;
  }

  const module = await import("./App.vue");
  createApp(module.default).use(i18n).mount("#app");
}

void bootstrap();
