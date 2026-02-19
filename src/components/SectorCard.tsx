import "./SectorCard.css";

export interface Sector {
  name: string;
  symbol: string;
}

interface SectorCardProps {
  sector: Sector;
  sectorRefreshing: boolean;
  anyRefreshing: boolean;
  onSectorRefresh: (symbol: string) => void;
}

function SectorCard({ sector, sectorRefreshing, anyRefreshing, onSectorRefresh }: SectorCardProps) {
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
          <span className="metric-value placeholder">--</span>
        </div>
        <div className="metric">
          <span className="metric-label">Outliers</span>
          <span className="metric-value placeholder">--</span>
        </div>
      </div>
    </div>
  );
}

export default SectorCard;
