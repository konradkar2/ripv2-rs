const routesBody = document.getElementById("routes-body");
const statusElement = document.getElementById("status");
const localRouteForm = document.getElementById("local-route-form");
const localRouteStatusElement = document.getElementById("local-route-status");
const logsOutput = document.getElementById("logs-output");
const logsStatusElement = document.getElementById("logs-status");
const sortButtons = Array.from(document.querySelectorAll(".sort-button"));
let routesCache = [];
let sortState = {
  key: "destination",
  direction: "asc",
};

const sortAccessors = {
  destination: (route) => [ipToNumber(route.destination), route.prefix],
  next_hop: (route) => ipToNumber(route.next_hop),
  metric: (route) => route.metric,
  interface_name: (route) => route.interface_name,
  route_type: (route) => route.route_type,
  state: (route) => `${route.state}${route.changed ? " changed" : ""}`,
  in_kernel: (route) => Number(route.in_kernel),
  timeout_seconds: (route) => route.timeout_seconds,
};

function routeLabel(route) {
  return `${route.destination}/${route.prefix}`;
}

function ipToNumber(address) {
  return address
    .split(".")
    .reduce((value, octet) => value * 256 + Number(octet), 0);
}

function compareValues(left, right) {
  if (Array.isArray(left) && Array.isArray(right)) {
    for (let index = 0; index < left.length; index += 1) {
      const result = compareValues(left[index], right[index]);
      if (result !== 0) {
        return result;
      }
    }

    return 0;
  }

  if (typeof left === "number" && typeof right === "number") {
    return left - right;
  }

  return String(left).localeCompare(String(right), undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function sortedRoutes(routes) {
  const accessor = sortAccessors[sortState.key];
  const direction = sortState.direction === "asc" ? 1 : -1;

  return [...routes].sort((left, right) => {
    const result = compareValues(accessor(left), accessor(right));
    if (result !== 0) {
      return result * direction;
    }

    return compareValues(sortAccessors.destination(left), sortAccessors.destination(right));
  });
}

function updateSortControls() {
  for (const button of sortButtons) {
    const key = button.dataset.sortKey;
    const indicator = button.querySelector(".sort-indicator");
    const header = button.closest("th");
    const isActive = key === sortState.key;

    button.dataset.active = String(isActive);
    indicator.textContent = isActive
      ? sortState.direction === "asc"
        ? "▲"
        : "▼"
      : "";
    header.setAttribute(
      "aria-sort",
      isActive
        ? sortState.direction === "asc"
          ? "ascending"
          : "descending"
        : "none",
    );
  }
}

function renderRoutes(routes) {
  routesBody.replaceChildren();
  updateSortControls();

  if (routes.length === 0) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.className = "empty";
    cell.colSpan = 8;
    cell.textContent = "No RIP routes";
    row.appendChild(cell);
    routesBody.appendChild(row);
    return;
  }

  for (const route of sortedRoutes(routes)) {
    const row = document.createElement("tr");
    const cells = [
      routeLabel(route),
      route.next_hop,
      route.metric,
      route.interface_name,
      route.route_type,
      route.changed ? `${route.state}, changed` : route.state,
      route.in_kernel ? "yes" : "no",
      `${route.timeout_seconds}s`,
    ];

    for (const value of cells) {
      const cell = document.createElement("td");
      cell.textContent = value;
      row.appendChild(cell);
    }

    routesBody.appendChild(row);
  }
}

async function loadRoutes() {
  try {
    const response = await fetch("/api/v1/routes", {
      headers: { Accept: "application/json" },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    routesCache = await response.json();
    renderRoutes(routesCache);
    statusElement.textContent = `Updated ${new Date().toLocaleTimeString()}`;
    statusElement.dataset.state = "ok";
  } catch (error) {
    statusElement.textContent = `Error: ${error.message}`;
    statusElement.dataset.state = "error";
  }
}

async function addLocalRoute(event) {
  event.preventDefault();
  const formData = new FormData(localRouteForm);
  const payload = {
    address: String(formData.get("address") ?? "").trim(),
    prefix: Number(formData.get("prefix")),
    dev: String(formData.get("dev") ?? "").trim(),
  };

  localRouteStatusElement.textContent = "Saving";
  localRouteStatusElement.dataset.state = "";

  try {
    const response = await fetch("/api/v1/local-routes", {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const message = await response.text();
      throw new Error(message || `HTTP ${response.status}`);
    }

    localRouteStatusElement.textContent = "Added";
    localRouteStatusElement.dataset.state = "ok";
    localRouteForm.reset();
    document.getElementById("local-route-prefix").value = "24";
    await loadRoutes();
    await loadLogs();
  } catch (error) {
    localRouteStatusElement.textContent = `Error: ${error.message}`;
    localRouteStatusElement.dataset.state = "error";
  }
}

async function loadLogs() {
  try {
    const response = await fetch("/api/v1/logs", {
      headers: { Accept: "text/plain" },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    logsOutput.textContent = await response.text();
    logsOutput.scrollTop = logsOutput.scrollHeight;
    logsStatusElement.textContent = `Updated ${new Date().toLocaleTimeString()}`;
    logsStatusElement.dataset.state = "ok";
  } catch (error) {
    logsStatusElement.textContent = `Error: ${error.message}`;
    logsStatusElement.dataset.state = "error";
  }
}

localRouteForm.addEventListener("submit", addLocalRoute);

for (const button of sortButtons) {
  button.addEventListener("click", () => {
    const key = button.dataset.sortKey;
    if (sortState.key === key) {
      sortState.direction = sortState.direction === "asc" ? "desc" : "asc";
    } else {
      sortState = { key, direction: "asc" };
    }

    renderRoutes(routesCache);
  });
}

loadRoutes();
loadLogs();
window.setInterval(loadRoutes, 3000);
window.setInterval(loadLogs, 3000);
