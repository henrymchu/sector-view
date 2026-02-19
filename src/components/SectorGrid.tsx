import type { SectorSummary, SectorOutliers } from "../types/database";
import SectorCard from "./SectorCard";
import "./SectorGrid.css";

interface SectorGridProps {
  sectors: SectorSummary[];
  outliersBySector: Map<string, SectorOutliers>;
  refreshingSectors: Set<string>;
  anyRefreshing: boolean;
  onSectorRefresh: (symbol: string) => void;
}

function SectorGrid({ sectors, outliersBySector, refreshingSectors, anyRefreshing, onSectorRefresh }: SectorGridProps) {
  return (
    <div className="sector-grid">
      {sectors.map((sector) => (
        <SectorCard
          key={sector.symbol}
          sector={sector}
          outliers={outliersBySector.get(sector.symbol)}
          sectorRefreshing={refreshingSectors.has(sector.symbol)}
          anyRefreshing={anyRefreshing}
          onSectorRefresh={onSectorRefresh}
        />
      ))}
    </div>
  );
}

export default SectorGrid;
