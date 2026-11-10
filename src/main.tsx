import { invoke, isTauri } from "@tauri-apps/api/core";
import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";

function App() {
  const desktop = isTauri();
  const [status, setStatus] = useState("");

  async function checkConnection() {
    try {
      setStatus(await invoke<string>("check_connection"));
    } catch (error) {
      setStatus(`Desktop connection failed: ${String(error)}`);
    }
  }

  return (
    <main>
      <h1>LibraDesk</h1>
      <p>Personal PDF library for Windows</p>
      {desktop ? (
        <button type="button" onClick={checkConnection}>
          Check desktop connection
        </button>
      ) : (
        <p>Browser preview. Run npm run tauri dev for desktop mode.</p>
      )}
      <p role="status">{status}</p>
    </main>
  );
}

const root = document.getElementById("root");
if (!root) {
  throw new Error("The application root is missing.");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
