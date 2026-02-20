import { invoke } from "@tauri-apps/api/core";
import type { Sector, Stock, SectorSummary, RefreshResult, SectorOutliers, OutlierStock, UniverseType } from "../types/database";

export function useDatabase() {
  const getSectors = async (): Promise<Sector[]> => {
    try {
      return await invoke<Sector[]>("get_sectors");
    } catch (error) {
      console.error("Failed to get sectors:", error);
      throw error;
    }
  };

  const getStocksBySector = async (sectorId: number): Promise<Stock[]> => {
    try {
      return await invoke<Stock[]>("get_stocks_by_sector", { sectorId });
    } catch (error) {
      console.error("Failed to get stocks:", error);
      throw error;
    }
  };

  const getSectorPerformance = async (universe?: UniverseType): Promise<SectorSummary[]> => {
    try {
      return await invoke<SectorSummary[]>("get_sector_performance", {
        universe: universe ?? null,
      });
    } catch (error) {
      console.error("Failed to get sector performance:", error);
      throw error;
    }
  };

  const refreshMarketData = async (): Promise<RefreshResult> => {
    try {
      return await invoke<RefreshResult>("refresh_market_data");
    } catch (error) {
      console.error("Failed to refresh market data:", error);
      throw error;
    }
  };

  const refreshSectorData = async (sectorSymbol: string): Promise<SectorSummary[]> => {
    try {
      return await invoke<SectorSummary[]>("refresh_sector_data", { sectorSymbol });
    } catch (error) {
      console.error("Failed to refresh sector data:", error);
      throw error;
    }
  };

  const detectOutliers = async (threshold?: number, universe?: UniverseType): Promise<SectorOutliers[]> => {
    try {
      return await invoke<SectorOutliers[]>("detect_outliers", {
        threshold: threshold ?? null,
        universe: universe ?? null,
      });
    } catch (error) {
      console.error("Failed to detect outliers:", error);
      throw error;
    }
  };

  const getSectorOutliers = async (sectorId: number, threshold?: number, universe?: UniverseType): Promise<OutlierStock[]> => {
    try {
      return await invoke<OutlierStock[]>("get_sector_outliers", {
        sectorId,
        threshold: threshold ?? null,
        universe: universe ?? null,
      });
    } catch (error) {
      console.error("Failed to get sector outliers:", error);
      throw error;
    }
  };

  const refreshRussell2000Data = async (): Promise<RefreshResult> => {
    try {
      return await invoke<RefreshResult>("refresh_russell_2000_data");
    } catch (error) {
      console.error("Failed to refresh Russell 2000 data:", error);
      throw error;
    }
  };

  return {
    getSectors,
    getStocksBySector,
    getSectorPerformance,
    refreshMarketData,
    refreshSectorData,
    detectOutliers,
    getSectorOutliers,
    refreshRussell2000Data,
  };
}
