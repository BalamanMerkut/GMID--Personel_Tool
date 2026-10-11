// Sadece Latin + Türkçe karakterler (diğer alfabeler paketi büyütmesin).
import "@fontsource/montserrat/latin-600.css";
import "@fontsource/montserrat/latin-ext-600.css";
import "@fontsource/montserrat/latin-700.css";
import "@fontsource/montserrat/latin-ext-700.css";
import "@fontsource/montserrat/latin-800.css";
import "@fontsource/montserrat/latin-ext-800.css";
import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-ext-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-ext-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-ext-600.css";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";

export default mount(App, { target: document.getElementById("app")! });
