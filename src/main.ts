import { createApp } from "vue";
import App from "./App.vue";
import SettingsWindow from "./SettingsWindow.vue";
import "./style.css";

const isSettingsWindow = window.location.hash === "#settings";
const RootComponent = isSettingsWindow ? SettingsWindow : App;

createApp(RootComponent).mount("#app");
