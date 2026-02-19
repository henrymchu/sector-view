import type { SectorSummary } from "../types/database";
import "./SectorCard.css";

interface SectorCardProps {
  sector: SectorSummary;
  sectorRefreshing: boolean;
  anyRefreshing: boolean;
  onSectorRefresh: (symbol: string) => void;
}

function formatPercent(value: number): string {
  const sign = value >= 0 ? "+" : "";
  return `${sign}${value.toFixed(2)}%`;
}

function formatMarketCap(value: number | null): string {
  if (value == null) return "--";
  if (value >= 1e12) return `$${(value / 1e12).toFixed(1)}T`;
  if (value >= 1e9) return `$${(value / 1e9).toFixed(1)}B`;
  if (value >= 1e6) return `$${(value / 1e6).toFixed(0)}M`;
  return `$${value}`;
}

function SectorCard({ sector, sectorRefreshing, anyRefreshing, onSectorRefresh }: SectorCardProps) {
  const hasData = sector.stock_count > 0 && sector.avg_change_percent !== 0;
  const changeClass = sector.avg_change_percent >= 0 ? "positive" : "negative";

  return (
    <div className="sector-card" tabIndex={0}>
      <div className="sector-header">
        <div className="sector-header-top">
          <div>
            <h3 className="sector-name">{sector.name}</h3>
            <span className="sector-symbol">{sector.symbol}</span>
          </div>
          <button
            className="sector-refresh-mini"
            onClick={(e) => {
              e.stopPropagation();
              onSectorRefresh(sector.symbol);
            }}
            disabled={anyRefreshing}
            aria-label={sectorRefreshing ? `Refreshing ${sector.name}` : `Refresh ${sector.name}`}
          >
            <svg
              className={`mini-refresh-icon ${sectorRefreshing ? "spin" : ""}`}
              viewBox="0 0 24 24"
              width="14"
              height="14"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              {sectorRefreshing ? (
                <path d="M21 12a9 9 0 1 1-6.219-8.56" />
              ) : (
                <>
                  <path d="M21 12a9 9 0 0 1-15.36 6.36M3 12a9 9 0 0 1 15.36-6.36" />
                  <path d="M21 3v9h-9M3 21v-9h9" />
                </>
              )}
            </svg>
          </button>
        </div>
      </div>
      <div className="sector-metrics">
        <div className="metric">
          <span className="metric-label">Change</span>
          <span className={`metric-value ${hasData ? changeClass : "placeholder"}`}>
            {hasData ? formatPercent(sector.avg_change_percent) : "--"}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">Avg P/E</span>
          <span className={`metric-value ${sector.avg_pe_ratio != null ? "" : "placeholder"}`}>
            {sector.avg_pe_ratio != null ? sector.avg_pe_ratio.toFixed(1) : "--"}
          </span>
        </div>
        <div className="metric">
          <span className="metric-label">Mkt Cap</span>
          <span className={`metric-value ${sector.total_market_cap != null ? "" : "placeholder"}`}>
            {formatMarketCap(sector.total_market_cap)}
          </span>
        </div>
      </div>
    </div>
  );
}

export default SectorCard;
