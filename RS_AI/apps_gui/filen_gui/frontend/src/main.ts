// filen_gui frontend — Phase 1 scaffold.
// Placeholder bootstrap: wires up nav tabs, sidebar, and the transfer drawer.
// Real state management (store.ts) and Tauri command bindings land in later phases.

function bindNavTabs(): void {
  const tabs = document.querySelectorAll<HTMLButtonElement>(".nav-tab");
  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      tabs.forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");
      // Phase 3+: switch central view (Explorer/Recents/Sync/Servers).
      console.log(`[nav] view → ${tab.dataset.view}`);
    });
  });
}

function bindSidebar(): void {
  const items = document.querySelectorAll<HTMLButtonElement>(".side-item[data-view]");
  items.forEach((item) => {
    item.addEventListener("click", () => {
      items.forEach((i) => i.classList.remove("active"));
      item.classList.add("active");
      console.log(`[sidebar] view → ${item.dataset.view}`);
    });
  });

  document.getElementById("btn-login")?.addEventListener("click", () => {
    console.log("[auth] open login modal (Phase 2+)");
  });
}

function bindDrawer(): void {
  const toggle = document.getElementById("drawer-toggle");
  const body = document.getElementById("drawer-body");
  if (!toggle || !body) return;
  toggle.addEventListener("click", () => {
    const hidden = body.style.display === "none";
    body.style.display = hidden ? "block" : "none";
    const label = document.getElementById("drawer-label");
    if (label) {
      label.textContent = hidden
        ? "⬇️ Transfer (0) — bấm để đóng"
        : "⬇️ Transfer (0) — bấm để mở";
    }
  });
}

function main(): void {
  bindNavTabs();
  bindSidebar();
  bindDrawer();
}

main();