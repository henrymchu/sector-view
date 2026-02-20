import type { SectorSummary, SectorOutliers } from "../../types/database";

export const mockSector: SectorSummary = {
  sector_id: 1,
  name: "Information Technology",
  symbol: "XLK",
  avg_change_percent: 2.1,
  avg_pe_ratio: 28.5,
  total_market_cap: 12_500_000_000_000, // $12.5T
  stock_count: 65,
  avg_beta: 1.2,
};

export const mockSectorNoData: SectorSummary = {
  sector_id: 2,
  name: "Energy",
  symbol: "XLE",
  avg_change_percent: 0,
  avg_pe_ratio: null,
  total_market_cap: null,
  stock_count: 0,
  avg_beta: null,
};

export const mockSectorNegativeChange: SectorSummary = {
  sector_id: 3,
  name: "Utilities",
  symbol: "XLU",
  avg_change_percent: -1.5,
  avg_pe_ratio: 18.2,
  total_market_cap: 500_000_000_000, // $500.0B
  stock_count: 28,
  avg_beta: 0.6,
};

export const mockSectorMillions: SectorSummary = {
  sector_id: 5,
  name: "Real Estate",
  symbol: "XLRE",
  avg_change_percent: 0.5,
  avg_pe_ratio: 22.0,
  total_market_cap: 750_000_000, // $750M
  stock_count: 30,
  avg_beta: 0.9,
};

export const mockOutliers: SectorOutliers = {
  sector_id: 1,
  sector_name: "Information Technology",
  sector_symbol: "XLK",
  outlier_count: 2,
  outliers: [
    {
      stock_id: 1,
      symbol: "AAPL",
      name: "Apple Inc.",
      z_scores: { pe_z: 2.5, pb_z: 1.8, price_z: 2.1, volume_z: 1.2 },
      composite_score: 2.1,
      outlier_type: "GrowthPremium",
      significance_level: "Strong",
    },
    {
      stock_id: 2,
      symbol: "MSFT",
      name: "Microsoft Corporation",
      z_scores: { pe_z: -2.1, pb_z: -1.8, price_z: -1.5, volume_z: null },
      composite_score: 1.8,
      outlier_type: "Undervalued",
      significance_level: "Moderate",
    },
  ],
};

export const mockOutliersSingle: SectorOutliers = {
  sector_id: 3,
  sector_name: "Utilities",
  sector_symbol: "XLU",
  outlier_count: 1,
  outliers: [
    {
      stock_id: 10,
      symbol: "NEE",
      name: "NextEra Energy",
      z_scores: { pe_z: 3.1, pb_z: 2.4, price_z: 1.8, volume_z: null },
      composite_score: 2.5,
      outlier_type: "Overvalued",
      significance_level: "Strong",
    },
  ],
};

export const mockOutliersEmpty: SectorOutliers = {
  sector_id: 2,
  sector_name: "Energy",
  sector_symbol: "XLE",
  outlier_count: 0,
  outliers: [],
};

export const mockSectors: SectorSummary[] = [
  mockSector,
  mockSectorNoData,
  mockSectorNegativeChange,
  {
    sector_id: 4,
    name: "Financials",
    symbol: "XLF",
    avg_change_percent: 0.8,
    avg_pe_ratio: 15.2,
    total_market_cap: 8_000_000_000_000,
    stock_count: 72,
    avg_beta: 1.1,
  },
];
