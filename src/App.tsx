import { useState, useCallback } from "react";
import Header from "./components/Header";
import SectorGrid, { SECTORS } from "./components/SectorGrid";
import Toast, { type ToastMessage } from "./components/Toast";
import "./App.css";

let toastId = 0;

function App() {
  const [globalRefreshing, setGlobalRefreshing] = useState(false);
  const [refreshingSectors, setRefreshingSectors] = useState<Set<string>>(new Set());
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const anyRefreshing = globalRefreshing || refreshingSectors.size > 0;

  const showToast = useCallback((text: string, type: "success" | "error") => {
    const id = ++toastId;
    setToasts((prev) => [...prev, { id, text, type }]);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const handleGlobalRefresh = useCallback(async () => {
    if (anyRefreshing) return;
    setGlobalRefreshing(true);
    try {
      // Simulate refreshing all stocks + discovery
      await new Promise((resolve) => setTimeout(resolve, 1500));
      setLastRefresh(new Date());
      const stockCount = SECTORS.length * 5; // Mock count
      showToast(`Updated ${stockCount} stocks, found 0 new stocks`, "success");
    } catch {
      showToast("Failed to refresh data", "error");
    } finally {
      setGlobalRefreshing(false);
    }
  }, [anyRefreshing, showToast]);

  const handleSectorRefresh = useCallback(async (symbol: string) => {
    if (anyRefreshing) return;
    setRefreshingSectors(new Set([symbol]));
    try {
      // Simulate refreshing a single sector
      await new Promise((resolve) => setTimeout(resolve, 1000));
      const sector = SECTORS.find((s) => s.symbol === symbol);
      const mockCount = Math.floor(Math.random() * 10) + 5;
      showToast(`Updated ${mockCount} ${sector?.name ?? symbol} stocks`, "success");
    } catch {
      const sector = SECTORS.find((s) => s.symbol === symbol);
      showToast(`Failed to refresh ${sector?.name ?? symbol}`, "error");
    } finally {
      setRefreshingSectors(new Set());
    }
  }, [anyRefreshing, showToast]);

  return (
    <>
      <Header
        refreshing={globalRefreshing}
        lastRefresh={lastRefresh}
        onRefresh={handleGlobalRefresh}
      />
      <main className="container">
        <SectorGrid
          refreshingSectors={refreshingSectors}
          anyRefreshing={anyRefreshing}
          onSectorRefresh={handleSectorRefresh}
        />
      </main>
      <Toast toasts={toasts} onDismiss={dismissToast} />
    </>
  );
}

export default App;
