import type { SectorSummary } from "../types/database";
import SectorCard from "./SectorCard";
import "./SectorGrid.css";

interface SectorGridProps {
  sectors: SectorSummary[];
  refreshingSectors: Set<string>;
  anyRefreshing: boolean;
  onSectorRefresh: (symbol: string) => void;
}

function SectorGrid({ sectors, refreshingSectors, anyRefreshing, onSectorRefresh }: SectorGridProps) {
  return (
    <div className="sector-grid">
      {sectors.map((sector) => (
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
