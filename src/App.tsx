import { useState, useCallback, useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Header from "./components/Header";
import SectorGrid from "./components/SectorGrid";
import OutlierDashboard from "./components/OutlierDashboard";
import Toast, { type ToastMessage } from "./components/Toast";
import { useDatabase } from "./hooks/useDatabase";
import type { SectorSummary, SectorOutliers } from "./types/database";
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

function toOutlierMap(data: SectorOutliers[]): Map<string, SectorOutliers> {
  const map = new Map<string, SectorOutliers>();
  for (const so of data) {
    map.set(so.sector_symbol, so);
  }
  return map;
}

function App() {
  const [sectors, setSectors] = useState<SectorSummary[]>(DEFAULT_SECTORS);
  const [outliersBySector, setOutliersBySector] = useState<Map<string, SectorOutliers>>(new Map());
  const [globalRefreshing, setGlobalRefreshing] = useState(false);
  const [refreshingSectors, setRefreshingSectors] = useState<Set<string>>(new Set());
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [progress, setProgress] = useState<{ current: number; total: number; phase: string } | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const { getSectorPerformance, refreshMarketData, refreshSectorData, detectOutliers } = useDatabase();

  const anyRefreshing = globalRefreshing || refreshingSectors.size > 0;

  const showToast = useCallback((text: string, type: "success" | "error") => {
    const id = ++toastId;
    setToasts((prev) => [...prev, { id, text, type }]);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const loadOutliers = useCallback(async () => {
    try {
      const data = await detectOutliers();
      setOutliersBySector(toOutlierMap(data));
    } catch {
      // Outlier detection is non-critical; keep existing data
    }
  }, [detectOutliers]);

  // Load cached/existing sector performance and outliers on mount
  useEffect(() => {
    getSectorPerformance()
      .then((data) => {
        if (data.length > 0) {
          setSectors(data);
          // Load outliers after performance data is available
          loadOutliers();
        }
      })
      .catch(() => {
        // Keep default sectors on error
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleGlobalRefresh = useCallback(async () => {
    if (anyRefreshing) return;
    setGlobalRefreshing(true);
    setProgress(null);
    try {
      unlistenRef.current = await listen<{ current: number; total: number; phase: string }>(
        "refresh-progress",
        (event) => setProgress(event.payload),
      );
      const result = await refreshMarketData();
      setSectors(result.sectors);
      setLastRefresh(new Date());
      const totalStocks = result.sectors.reduce((sum, s) => sum + s.stock_count, 0);
      let message = `Updated ${totalStocks} stocks across all sectors`;
      if (result.discovery) {
        const d = result.discovery;
        if (d.stocks_discovered > 0 || d.stocks_updated > 0) {
          message += ` | Found ${d.stocks_discovered} new, ${d.stocks_updated} sector changes`;
        }
      }
      showToast(message, "success");
      // Re-run outlier detection with fresh data
      await loadOutliers();
    } catch {
      showToast("Failed to refresh market data", "error");
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
      setProgress(null);
      setGlobalRefreshing(false);
    }
  }, [anyRefreshing, refreshMarketData, showToast, loadOutliers]);

  const handleSectorRefresh = useCallback(async (symbol: string) => {
    if (anyRefreshing) return;
    setRefreshingSectors(new Set([symbol]));
    setProgress(null);
    try {
      unlistenRef.current = await listen<{ current: number; total: number; phase: string }>(
        "refresh-progress",
        (event) => setProgress(event.payload),
      );
      const data = await refreshSectorData(symbol);
      setSectors(data);
      const sector = data.find((s) => s.symbol === symbol);
      showToast(`Updated ${sector?.stock_count ?? 0} ${sector?.name ?? symbol} stocks`, "success");
      // Re-run outlier detection
      await loadOutliers();
    } catch {
      const sector = sectors.find((s) => s.symbol === symbol);
      showToast(`Failed to refresh ${sector?.name ?? symbol}`, "error");
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
      setProgress(null);
      setRefreshingSectors(new Set());
    }
  }, [anyRefreshing, refreshSectorData, showToast, sectors, loadOutliers]);

  return (
    <>
      <Header
        refreshing={anyRefreshing}
        lastRefresh={lastRefresh}
        onRefresh={handleGlobalRefresh}
        progress={progress}
      />
      <main className="container">
        <SectorGrid
          sectors={sectors}
          outliersBySector={outliersBySector}
          refreshingSectors={refreshingSectors}
          anyRefreshing={anyRefreshing}
          onSectorRefresh={handleSectorRefresh}
        />
        <OutlierDashboard outliersBySector={outliersBySector} />
      </main>
      <Toast toasts={toasts} onDismiss={dismissToast} />
    </>
  );
}

export default App;
