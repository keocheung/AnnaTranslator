import { createApp } from "vue";
import App from "./App.vue";
import SettingsWindow from "./SettingsWindow.vue";
import HistoryWindow from "./HistoryWindow.vue";
import "./style.css";

const isSettingsWindow = window.location.hash === "#settings";
const isHistoryWindow = window.location.hash === "#history";
const RootComponent = isSettingsWindow ? SettingsWindow : isHistoryWindow ? HistoryWindow : App;

createApp(RootComponent).mount("#app");
