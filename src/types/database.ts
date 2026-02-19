export interface Sector {
  id: number;
  name: string;
  symbol: string;
}

export interface Stock {
  id: number;
  symbol: string;
  name: string;
  sector_id: number | null;
}

export interface SectorSummary {
  sector_id: number;
  name: string;
  symbol: string;
  avg_change_percent: number;
  avg_pe_ratio: number | null;
  total_market_cap: number | null;
  stock_count: number;
  avg_beta: number | null;
}
