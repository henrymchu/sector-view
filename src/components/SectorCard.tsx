import "./SectorCard.css";

export interface Sector {
  name: string;
  symbol: string;
}

interface SectorCardProps {
  sector: Sector;
}

function SectorCard({ sector }: SectorCardProps) {
  return (
    <div className="sector-card" tabIndex={0}>
      <div className="sector-header">
        <h3 className="sector-name">{sector.name}</h3>
        <span className="sector-symbol">{sector.symbol}</span>
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
