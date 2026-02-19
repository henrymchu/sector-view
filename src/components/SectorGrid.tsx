import SectorCard, { type Sector } from "./SectorCard";
import "./SectorGrid.css";

export const SECTORS: Sector[] = [
  { name: "Technology", symbol: "XLK" },
  { name: "Health Care", symbol: "XLV" },
  { name: "Financials", symbol: "XLF" },
  { name: "Consumer Discretionary", symbol: "XLY" },
  { name: "Communication Services", symbol: "XLC" },
  { name: "Industrials", symbol: "XLI" },
  { name: "Consumer Staples", symbol: "XLP" },
  { name: "Energy", symbol: "XLE" },
  { name: "Utilities", symbol: "XLU" },
  { name: "Real Estate", symbol: "XLRE" },
  { name: "Materials", symbol: "XLB" },
];

interface SectorGridProps {
  refreshingSectors: Set<string>;
  anyRefreshing: boolean;
  onSectorRefresh: (symbol: string) => void;
}

function SectorGrid({ refreshingSectors, anyRefreshing, onSectorRefresh }: SectorGridProps) {
  return (
    <div className="sector-grid">
      {SECTORS.map((sector) => (
        <SectorCard
          key={sector.symbol}
          sector={sector}
          sectorRefreshing={refreshingSectors.has(sector.symbol)}
          anyRefreshing={anyRefreshing}
          onSectorRefresh={onSectorRefresh}
        />
      ))}
    </div>
  );
}

export default SectorGrid;
