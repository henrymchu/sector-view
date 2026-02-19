import { useState, useCallback, useEffect } from "react";
import Header from "./components/Header";
import SectorGrid from "./components/SectorGrid";
import Toast, { type ToastMessage } from "./components/Toast";
import { useDatabase } from "./hooks/useDatabase";
import type { SectorSummary } from "./types/database";
import "./App.css";

// Default sector data shown before any data is loaded
const DEFAULT_SECTORS: SectorSummary[] = [
  { sector_id: 0, name: "Technology", symbol: "XLK", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Health Care", symbol: "XLV", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Financials", symbol: "XLF", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Consumer Discretionary", symbol: "XLY", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Communication Services", symbol: "XLC", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Industrials", symbol: "XLI", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Consumer Staples", symbol: "XLP", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Energy", symbol: "XLE", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Utilities", symbol: "XLU", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Real Estate", symbol: "XLRE", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
  { sector_id: 0, name: "Materials", symbol: "XLB", avg_change_percent: 0, avg_pe_ratio: null, total_market_cap: null, stock_count: 0, avg_beta: null },
];

let toastId = 0;

function App() {
  const [sectors, setSectors] = useState<SectorSummary[]>(DEFAULT_SECTORS);
  const [globalRefreshing, setGlobalRefreshing] = useState(false);
  const [refreshingSectors, setRefreshingSectors] = useState<Set<string>>(new Set());
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const { getSectorPerformance, refreshMarketData, refreshSectorData } = useDatabase();

  const anyRefreshing = globalRefreshing || refreshingSectors.size > 0;

  const showToast = useCallback((text: string, type: "success" | "error") => {
    const id = ++toastId;
    setToasts((prev) => [...prev, { id, text, type }]);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  // Load cached/existing sector performance on mount
  useEffect(() => {
    getSectorPerformance()
      .then((data) => {
        if (data.length > 0) {
          setSectors(data);
        }
      })
      .catch(() => {
        // Keep default sectors on error
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleGlobalRefresh = useCallback(async () => {
    if (anyRefreshing) return;
    setGlobalRefreshing(true);
    try {
      const data = await refreshMarketData();
      setSectors(data);
      setLastRefresh(new Date());
      const totalStocks = data.reduce((sum, s) => sum + s.stock_count, 0);
      showToast(`Updated ${totalStocks} stocks across all sectors`, "success");
    } catch {
      showToast("Failed to refresh market data", "error");
    } finally {
      setGlobalRefreshing(false);
    }
  }, [anyRefreshing, refreshMarketData, showToast]);

  const handleSectorRefresh = useCallback(async (symbol: string) => {
    if (anyRefreshing) return;
    setRefreshingSectors(new Set([symbol]));
    try {
      const data = await refreshSectorData(symbol);
      setSectors(data);
      const sector = data.find((s) => s.symbol === symbol);
      showToast(`Updated ${sector?.stock_count ?? 0} ${sector?.name ?? symbol} stocks`, "success");
    } catch {
      const sector = sectors.find((s) => s.symbol === symbol);
      showToast(`Failed to refresh ${sector?.name ?? symbol}`, "error");
    } finally {
      setRefreshingSectors(new Set());
    }
  }, [anyRefreshing, refreshSectorData, showToast, sectors]);

  return (
    <>
      <Header
        refreshing={globalRefreshing}
        lastRefresh={lastRefresh}
        onRefresh={handleGlobalRefresh}
      />
      <main className="container">
        <SectorGrid
          sectors={sectors}
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
