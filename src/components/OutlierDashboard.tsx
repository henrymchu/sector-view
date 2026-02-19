import { useState, useMemo } from "react";
import type { SectorOutliers, OutlierStock, OutlierType, SignificanceLevel } from "../types/database";
import "./OutlierDashboard.css";

type SortField = "composite" | "pe" | "pb" | "price" | "volume";

interface OutlierDashboardProps {
  outliersBySector: Map<string, SectorOutliers>;
}

function getAllOutliers(outliersBySector: Map<string, SectorOutliers>): (OutlierStock & { sectorName: string; sectorSymbol: string })[] {
  const all: (OutlierStock & { sectorName: string; sectorSymbol: string })[] = [];
  for (const so of outliersBySector.values()) {
    for (const o of so.outliers) {
      all.push({ ...o, sectorName: so.sector_name, sectorSymbol: so.sector_symbol });
    }
  }
  return all;
}

function sortOutliers(
  outliers: (OutlierStock & { sectorName: string; sectorSymbol: string })[],
  field: SortField,
): (OutlierStock & { sectorName: string; sectorSymbol: string })[] {
  const sorted = [...outliers];
  sorted.sort((a, b) => {
    switch (field) {
      case "composite": return b.composite_score - a.composite_score;
      case "pe": return Math.abs(b.z_scores.pe_z ?? 0) - Math.abs(a.z_scores.pe_z ?? 0);
      case "pb": return Math.abs(b.z_scores.pb_z ?? 0) - Math.abs(a.z_scores.pb_z ?? 0);
      case "price": return Math.abs(b.z_scores.price_z) - Math.abs(a.z_scores.price_z);
      case "volume": return Math.abs(b.z_scores.volume_z ?? 0) - Math.abs(a.z_scores.volume_z ?? 0);
      default: return 0;
    }
  });
  return sorted;
}

function significanceClass(level: SignificanceLevel): string {
  switch (level) {
    case "Extreme": return "sig-extreme";
    case "Strong": return "sig-strong";
    default: return "sig-moderate";
  }
}

function typeClass(type: OutlierType): string {
  switch (type) {
    case "Undervalued": return "type-undervalued";
    case "Overvalued": return "type-overvalued";
    case "Momentum": return "type-momentum";
    case "ValueTrap": return "type-valuetrap";
    case "GrowthPremium": return "type-growth";
    default: return "type-mixed";
  }
}

function formatZ(value: number | null): string {
  if (value == null) return "--";
  const sign = value >= 0 ? "+" : "";
  return `${sign}${value.toFixed(1)}`;
}

function OutlierDashboard({ outliersBySector }: OutlierDashboardProps) {
  const [sortField, setSortField] = useState<SortField>("composite");
  const [filterType, setFilterType] = useState<OutlierType | "All">("All");
  const [filterSignificance, setFilterSignificance] = useState<SignificanceLevel | "All">("All");
  const [filterSector, setFilterSector] = useState<string>("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedId, setExpandedId] = useState<number | null>(null);

  const allOutliers = useMemo(() => getAllOutliers(outliersBySector), [outliersBySector]);

  const sectorNames = useMemo(() => {
    const names = new Set<string>();
    for (const so of outliersBySector.values()) {
      names.add(so.sector_name);
    }
    return Array.from(names).sort();
  }, [outliersBySector]);

  const filtered = useMemo(() => {
    let result = allOutliers;
    if (filterType !== "All") {
      result = result.filter((o) => o.outlier_type === filterType);
    }
    if (filterSignificance !== "All") {
      result = result.filter((o) => o.significance_level === filterSignificance);
    }
    if (filterSector !== "All") {
      result = result.filter((o) => o.sectorName === filterSector);
    }
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter((o) => o.symbol.toLowerCase().includes(q) || o.name.toLowerCase().includes(q));
    }
    return sortOutliers(result, sortField);
  }, [allOutliers, filterType, filterSignificance, filterSector, searchQuery, sortField]);

  if (allOutliers.length === 0) {
    return (
      <div className="outlier-dashboard">
        <h2 className="dashboard-title">Outlier Analysis</h2>
        <div className="dashboard-empty">
          No outliers detected. Refresh market data and try again.
        </div>
      </div>
    );
  }

  return (
    <div className="outlier-dashboard">
      <h2 className="dashboard-title">
        Outlier Analysis
        <span className="dashboard-count">{filtered.length} outlier{filtered.length !== 1 ? "s" : ""}</span>
      </h2>

      <div className="dashboard-controls">
        <input
          className="dashboard-search"
          type="text"
          placeholder="Search symbol or name..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />

        <div className="dashboard-filters">
          <select value={sortField} onChange={(e) => setSortField(e.target.value as SortField)}>
            <option value="composite">Sort: Composite</option>
            <option value="pe">Sort: P/E Z-Score</option>
            <option value="pb">Sort: P/B Z-Score</option>
            <option value="price">Sort: Price Z-Score</option>
            <option value="volume">Sort: Volume Z-Score</option>
          </select>

          <select value={filterType} onChange={(e) => setFilterType(e.target.value as OutlierType | "All")}>
            <option value="All">Type: All</option>
            <option value="Undervalued">Undervalued</option>
            <option value="Overvalued">Overvalued</option>
            <option value="Momentum">Momentum</option>
            <option value="ValueTrap">Value Trap</option>
            <option value="GrowthPremium">Growth Premium</option>
            <option value="Mixed">Mixed</option>
          </select>

          <select value={filterSignificance} onChange={(e) => setFilterSignificance(e.target.value as SignificanceLevel | "All")}>
            <option value="All">Significance: All</option>
            <option value="Moderate">Moderate (1.5&sigma;+)</option>
            <option value="Strong">Strong (2&sigma;+)</option>
            <option value="Extreme">Extreme (3&sigma;+)</option>
          </select>

          <select value={filterSector} onChange={(e) => setFilterSector(e.target.value)}>
            <option value="All">Sector: All</option>
            {sectorNames.map((name) => (
              <option key={name} value={name}>{name}</option>
            ))}
          </select>
        </div>
      </div>

      <div className="outlier-list">
        {filtered.map((outlier) => (
          <div
            key={`${outlier.stock_id}-${outlier.sectorSymbol}`}
            className={`outlier-item ${typeClass(outlier.outlier_type)} ${expandedId === outlier.stock_id ? "expanded" : ""}`}
            onClick={() => setExpandedId(expandedId === outlier.stock_id ? null : outlier.stock_id)}
          >
            <div className="outlier-row">
              <div className="outlier-stock-info">
                <span className="outlier-stock-symbol">{outlier.symbol}</span>
                <span className="outlier-stock-name">{outlier.name}</span>
                <span className="outlier-sector-badge">{outlier.sectorName}</span>
              </div>

              <div className={`outlier-composite ${significanceClass(outlier.significance_level)}`}>
                <span className="composite-value">{outlier.composite_score.toFixed(1)}&sigma;</span>
                <span className="composite-label">{outlier.significance_level}</span>
              </div>

              <div className="outlier-zscores">
                <div className="zscore-item">
                  <span className="zscore-label">P/E</span>
                  <span className={`zscore-value ${zClass(outlier.z_scores.pe_z)}`}>{formatZ(outlier.z_scores.pe_z)}</span>
                </div>
                <div className="zscore-item">
                  <span className="zscore-label">P/B</span>
                  <span className={`zscore-value ${zClass(outlier.z_scores.pb_z)}`}>{formatZ(outlier.z_scores.pb_z)}</span>
                </div>
                <div className="zscore-item">
                  <span className="zscore-label">Price</span>
                  <span className={`zscore-value ${zClass(outlier.z_scores.price_z)}`}>{formatZ(outlier.z_scores.price_z)}</span>
                </div>
                <div className="zscore-item">
                  <span className="zscore-label">Vol</span>
                  <span className={`zscore-value ${zClass(outlier.z_scores.volume_z)}`}>{formatZ(outlier.z_scores.volume_z)}</span>
                </div>
              </div>

              <div className="outlier-classification">
                <span className={`outlier-type-badge ${typeClass(outlier.outlier_type)}`}>
                  {outlier.outlier_type}
                </span>
              </div>
            </div>

            {expandedId === outlier.stock_id && (
              <div className="outlier-detail">
                <div className="detail-section">
                  <h4>Z-Score Analysis</h4>
                  <div className="detail-grid">
                    <DetailRow label="P/E Z-Score" value={outlier.z_scores.pe_z} description={peDescription(outlier.z_scores.pe_z)} />
                    <DetailRow label="P/B Z-Score" value={outlier.z_scores.pb_z} description={pbDescription(outlier.z_scores.pb_z)} />
                    <DetailRow label="Price Z-Score" value={outlier.z_scores.price_z} description={priceDescription(outlier.z_scores.price_z)} />
                    <DetailRow label="Volume Z-Score" value={outlier.z_scores.volume_z} description={volumeDescription(outlier.z_scores.volume_z)} />
                  </div>
                </div>
                <div className="detail-section">
                  <h4>Classification</h4>
                  <p className="detail-text">
                    <strong>{outlier.outlier_type}</strong> &mdash; {classificationDescription(outlier.outlier_type)}
                  </p>
                  <p className="detail-text">
                    Composite score of <strong>{outlier.composite_score.toFixed(2)}&sigma;</strong> indicates a <strong>{outlier.significance_level.toLowerCase()}</strong> outlier.
                  </p>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function zClass(value: number | null): string {
  if (value == null) return "";
  if (value >= 2) return "z-high";
  if (value <= -2) return "z-low";
  if (value >= 1) return "z-mid-high";
  if (value <= -1) return "z-mid-low";
  return "";
}

function DetailRow({ label, value, description }: { label: string; value: number | null; description: string }) {
  return (
    <div className="detail-row">
      <span className="detail-label">{label}</span>
      <span className={`detail-value ${zClass(value)}`}>{formatZ(value)}&sigma;</span>
      <span className="detail-desc">{description}</span>
    </div>
  );
}

function peDescription(z: number | null): string {
  if (z == null) return "No P/E data available";
  if (z <= -2) return "Significantly undervalued vs sector";
  if (z <= -1) return "Below sector average valuation";
  if (z >= 2) return "Significantly overvalued vs sector";
  if (z >= 1) return "Above sector average valuation";
  return "Near sector average valuation";
}

function pbDescription(z: number | null): string {
  if (z == null) return "No P/B data available";
  if (z <= -2) return "Trading well below book value vs sector";
  if (z <= -1) return "Below sector book value average";
  if (z >= 2) return "Trading well above book value vs sector";
  if (z >= 1) return "Above sector book value average";
  return "Near sector book value average";
}

function priceDescription(z: number): string {
  if (z >= 2) return "Strong outperformance vs sector";
  if (z >= 1) return "Moderate outperformance";
  if (z <= -2) return "Significant underperformance vs sector";
  if (z <= -1) return "Moderate underperformance";
  return "In line with sector performance";
}

function volumeDescription(z: number | null): string {
  if (z == null) return "No volume data available";
  if (z >= 2) return "Unusually high trading volume";
  if (z >= 1) return "Above-average trading activity";
  if (z <= -2) return "Unusually low trading volume";
  if (z <= -1) return "Below-average trading activity";
  return "Normal trading volume";
}

function classificationDescription(type: OutlierType): string {
  switch (type) {
    case "Undervalued": return "Low valuation multiples suggest stock may be underpriced relative to sector peers.";
    case "Overvalued": return "High valuation multiples suggest stock may be overpriced relative to sector peers.";
    case "Momentum": return "Strong price performance with high volume indicates momentum-driven movement.";
    case "ValueTrap": return "Low valuation with poor price performance may indicate fundamental challenges.";
    case "GrowthPremium": return "High valuation with strong price performance suggests market pricing in growth.";
    case "Mixed": return "Multiple factors contributing to outlier status without a dominant pattern.";
    default: return "";
  }
}

export default OutlierDashboard;
