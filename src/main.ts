import { createApp } from "vue";
import "./style.css";
import App from "./App.vue";
import { i18n, setLocale } from "./i18n";

setLocale(i18n.global.locale.value);
createApp(App).use(i18n).mount("#app");
