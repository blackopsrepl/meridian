(() => {
  "use strict";

  document.querySelectorAll("[data-sidebar-toggle]").forEach((button) => {
    button.addEventListener("click", () => document.body.classList.toggle("sidebar-open"));
  });

  const updateZonePanels = () => {
    const selected = document.querySelector("[data-zone-mode]:checked")?.value;
    document.querySelectorAll("[data-zone-panel]").forEach((panel) => {
      panel.classList.toggle("hidden", panel.dataset.zonePanel !== selected);
      panel.querySelectorAll("input, select").forEach((field) => {
        field.disabled = panel.dataset.zonePanel !== selected;
      });
    });
  };
  document.querySelectorAll("[data-zone-mode]").forEach((radio) => radio.addEventListener("change", updateZonePanels));
  updateZonePanels();

  document.querySelectorAll("[data-print]").forEach((button) => {
    button.addEventListener("click", () => window.print());
  });

  document.querySelectorAll("form[data-confirm]").forEach((form) => {
    form.addEventListener("submit", (event) => {
      if (!window.confirm(form.dataset.confirm)) event.preventDefault();
    });
  });

  const chartForm = document.querySelector("[data-chart-form]");
  if (chartForm) {
    const fold = chartForm.querySelector("select[name=fold]");
    chartForm.addEventListener("submit", () => {
      if (fold && fold.value === "") fold.disabled = true;
      const submit = chartForm.querySelector("button[type=submit]");
      if (submit) {
        submit.disabled = true;
        submit.textContent = "Reading ephemeris…";
      }
    });
  }

  const tabs = [...document.querySelectorAll(".chart-tabs a")];
  if (tabs.length && "IntersectionObserver" in window) {
    const targets = tabs.map((tab) => document.querySelector(tab.getAttribute("href"))).filter(Boolean);
    const observer = new IntersectionObserver((entries) => {
      const visible = entries.filter((entry) => entry.isIntersecting).sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
      if (!visible) return;
      tabs.forEach((tab) => tab.classList.toggle("active", tab.getAttribute("href") === `#${visible.target.id}`));
    }, { rootMargin: "-15% 0px -70%", threshold: [0, 0.2, 0.8] });
    targets.forEach((target) => observer.observe(target));
  }
})();

