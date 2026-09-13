const routesBody = document.getElementById("routes-body");
const statusElement = document.getElementById("status");

function routeLabel(route) {
  return `${route.destination}/${route.prefix}`;
}

function renderRoutes(routes) {
  routesBody.replaceChildren();

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

  for (const route of routes) {
    const row = document.createElement("tr");
    const cells = [
      routeLabel(route),
      route.next_hop,
      route.metric,
      route.interface_index,
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

    const routes = await response.json();
    renderRoutes(routes);
    statusElement.textContent = `Updated ${new Date().toLocaleTimeString()}`;
    statusElement.dataset.state = "ok";
  } catch (error) {
    statusElement.textContent = `Error: ${error.message}`;
    statusElement.dataset.state = "error";
  }
}

loadRoutes();
window.setInterval(loadRoutes, 3000);
