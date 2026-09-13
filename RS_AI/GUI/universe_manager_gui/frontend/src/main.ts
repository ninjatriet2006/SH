import "./style.css";
import { UniverseApp } from "./app";

const root = document.querySelector<HTMLElement>("#app");
if (!root) throw new Error("missing #app root");
const app = new UniverseApp(root);
void app.start();
window.addEventListener("beforeunload", () => app.dispose(), { once: true });
