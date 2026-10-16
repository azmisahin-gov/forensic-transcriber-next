import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./app/App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./styles/global.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Kök eleman bulunamadı (#root)");
}

createRoot(container).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
