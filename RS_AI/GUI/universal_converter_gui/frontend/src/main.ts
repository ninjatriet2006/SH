import "./style.css";
import { App } from "./app";

declare global {
  interface Window {
    __UNIVERSAL_CONVERTER_RESOURCES__?: import("./assets").ResourcePaths;
  }
}

const root = document.querySelector<HTMLElement>("#app");
if (!root) throw new Error("missing #app root");
void new App(root).start();
