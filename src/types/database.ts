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

export interface ZScores {
  pe_z: number | null;
  pb_z: number | null;
  price_z: number;
  volume_z: number | null;
}

export type OutlierType =
  | "Undervalued"
  | "Overvalued"
  | "Momentum"
  | "ValueTrap"
  | "GrowthPremium"
  | "Mixed";

export type SignificanceLevel = "Moderate" | "Strong" | "Extreme";

export interface OutlierStock {
  stock_id: number;
  symbol: string;
  name: string;
  z_scores: ZScores;
  composite_score: number;
  outlier_type: OutlierType;
  significance_level: SignificanceLevel;
}

export interface SectorOutliers {
  sector_id: number;
  sector_name: string;
  sector_symbol: string;
  outlier_count: number;
  outliers: OutlierStock[];
}
