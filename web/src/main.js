import { mount } from "svelte";
import "./estilo.css";
import App from "./App.svelte";
import { conectarAtras } from "./nativo.js";

conectarAtras();
mount(App, { target: document.getElementById("app") });
