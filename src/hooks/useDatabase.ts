import { invoke } from "@tauri-apps/api/core";
import type { Sector, Stock } from "../types/database";

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

  return { getSectors, getStocksBySector };
}
