import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DashboardPage } from "./pages/DashboardPage";
import type { AppStatus } from "./modules/app/types";

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppStatus>("get_app_status")
      .then(setStatus)
      .catch((cause) => setError(String(cause)));
  }, []);

  return <DashboardPage status={status} error={error} />;
}
