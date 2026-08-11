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
        if (field.name === "timezone") field.required = panel.dataset.zonePanel === selected;
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
    const cityPicker = chartForm.querySelector("[data-city-picker]");
    const citySearch = cityPicker?.querySelector("[data-city-search]");
    const cityId = cityPicker?.querySelector("[data-city-id]");
    const cityResults = cityPicker?.querySelector("[data-city-results]");
    const citySelection = cityPicker?.querySelector("[data-city-selection]");
    const cityName = cityPicker?.querySelector("[data-city-name]");
    const cityMeta = cityPicker?.querySelector("[data-city-meta]");
    const cityStatus = cityPicker?.querySelector("[data-city-status]");
    const cityClear = cityPicker?.querySelector("[data-city-clear]");
    const coordinateToggle = chartForm.querySelector("[data-manual-coordinates]");
    const coordinateFields = chartForm.querySelector("[data-coordinate-fields]");
    const timezoneToggle = chartForm.querySelector("[data-manual-timezone]");
    const timezoneFields = chartForm.querySelector("[data-timezone-fields]");
    const fold = chartForm.querySelector("select[name=fold]");

    const setOverrideState = (toggle, fields, requiredNames) => {
      if (!toggle || !fields) return;
      fields.disabled = !toggle.checked;
      fields.querySelectorAll("input").forEach((field) => {
        field.required = toggle.checked && requiredNames.includes(field.name);
      });
    };

    const updateOverrides = () => {
      setOverrideState(coordinateToggle, coordinateFields, ["location_name", "latitude", "longitude"]);
      setOverrideState(timezoneToggle, timezoneFields, []);
      updateZonePanels();
    };
    coordinateToggle?.addEventListener("change", updateOverrides);
    timezoneToggle?.addEventListener("change", updateOverrides);
    updateOverrides();

    let matches = [];
    let activeIndex = -1;
    let searchTimer;
    let searchRequest;

    const formatCoordinate = (value, positive, negative) => {
      const direction = value >= 0 ? positive : negative;
      return `${Math.abs(value).toFixed(4)}° ${direction}`;
    };

    const resultMeta = (city) => {
      const coordinates = `${formatCoordinate(city.latitude, "N", "S")} · ${formatCoordinate(city.longitude, "E", "W")}`;
      const population = city.population > 0 ? ` · ${new Intl.NumberFormat().format(city.population)} residents` : "";
      return `${city.timezone} · ${coordinates}${population}`;
    };

    const closeResults = () => {
      if (!cityResults || !citySearch) return;
      cityResults.hidden = true;
      citySearch.setAttribute("aria-expanded", "false");
      citySearch.removeAttribute("aria-activedescendant");
      activeIndex = -1;
    };

    const setActiveResult = (index) => {
      if (!cityResults || !citySearch || matches.length === 0) return;
      activeIndex = (index + matches.length) % matches.length;
      cityResults.querySelectorAll("[role=option]").forEach((option, optionIndex) => {
        const active = optionIndex === activeIndex;
        option.classList.toggle("active", active);
        option.setAttribute("aria-selected", String(active));
        if (active) {
          citySearch.setAttribute("aria-activedescendant", option.id);
          option.scrollIntoView({ block: "nearest" });
        }
      });
    };

    const selectCity = (city) => {
      if (!citySearch || !cityId || !citySelection || !cityName || !cityMeta || !cityStatus) return;
      citySearch.value = city.display_name;
      citySearch.setCustomValidity("");
      cityId.value = String(city.id);
      cityName.textContent = city.display_name;
      cityMeta.textContent = resultMeta(city);
      citySelection.hidden = false;
      cityStatus.textContent = "Atlas location selected. Coordinates and time zone will be applied automatically.";
      const values = {
        location_name: city.display_name,
        latitude: city.latitude.toFixed(6),
        longitude: city.longitude.toFixed(6),
        elevation_m: String(city.elevation_m),
        timezone: city.timezone,
      };
      Object.entries(values).forEach(([name, value]) => {
        const field = chartForm.querySelector(`[name=${name}]`);
        if (field) field.value = value;
      });
      closeResults();
    };

    const clearCity = (clearSearch = true) => {
      if (!cityId || !citySelection || !cityStatus) return;
      cityId.value = "";
      citySelection.hidden = true;
      cityStatus.textContent = "Choose one result from the local atlas.";
      if (clearSearch && citySearch) {
        citySearch.value = "";
        citySearch.focus();
      }
    };

    const renderMatches = () => {
      if (!cityResults || !citySearch || !cityStatus) return;
      cityResults.replaceChildren();
      if (matches.length === 0) {
        cityStatus.textContent = "No matching city. Try an alternate spelling or use the advanced manual override.";
        closeResults();
        return;
      }
      matches.forEach((city, index) => {
        const option = document.createElement("button");
        const title = document.createElement("strong");
        const meta = document.createElement("small");
        option.type = "button";
        option.id = `city-result-${index}`;
        option.className = "city-result";
        option.setAttribute("role", "option");
        option.setAttribute("aria-selected", "false");
        title.textContent = city.display_name;
        meta.textContent = resultMeta(city);
        option.append(title, meta);
        option.addEventListener("click", () => selectCity(city));
        cityResults.append(option);
      });
      cityResults.hidden = false;
      citySearch.setAttribute("aria-expanded", "true");
      cityStatus.textContent = `${matches.length} matching ${matches.length === 1 ? "city" : "cities"}.`;
      setActiveResult(0);
    };

    const searchCities = async (query) => {
      searchRequest?.abort();
      searchRequest = new AbortController();
      if (cityStatus) cityStatus.textContent = "Searching the local city atlas…";
      try {
        const response = await fetch(`/api/v1/locations?q=${encodeURIComponent(query)}&limit=8`, {
          signal: searchRequest.signal,
          headers: { Accept: "application/json" },
        });
        if (!response.ok) throw new Error(`city search returned ${response.status}`);
        matches = await response.json();
        renderMatches();
      } catch (error) {
        if (error.name === "AbortError") return;
        matches = [];
        closeResults();
        if (cityStatus) cityStatus.textContent = "City search failed. The advanced manual override remains available.";
      }
    };

    citySearch?.addEventListener("input", () => {
      clearCity(false);
      citySearch.setCustomValidity("");
      window.clearTimeout(searchTimer);
      const query = citySearch.value.trim();
      if (query.length < 2) {
        matches = [];
        closeResults();
        if (cityStatus) cityStatus.textContent = "Type at least two characters.";
        return;
      }
      searchTimer = window.setTimeout(() => searchCities(query), 160);
    });

    citySearch?.addEventListener("keydown", (event) => {
      if (cityResults?.hidden || matches.length === 0) return;
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActiveResult(activeIndex + 1);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        setActiveResult(activeIndex - 1);
      } else if (event.key === "Enter") {
        event.preventDefault();
        selectCity(matches[Math.max(activeIndex, 0)]);
      } else if (event.key === "Escape") {
        closeResults();
      }
    });

    cityClear?.addEventListener("click", () => clearCity(true));
    document.addEventListener("click", (event) => {
      if (cityPicker && !cityPicker.contains(event.target)) closeResults();
    });

    chartForm.addEventListener("submit", (event) => {
      const cityNeeded = !coordinateToggle?.checked || !timezoneToggle?.checked;
      if (cityNeeded && !cityId?.value) {
        event.preventDefault();
        if (citySearch) {
          citySearch.setCustomValidity("Choose a city from the atlas, or enable both manual overrides.");
          citySearch.reportValidity();
          citySearch.focus();
        }
        if (cityStatus) cityStatus.textContent = "Select a city result before casting the chart.";
        return;
      }
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
