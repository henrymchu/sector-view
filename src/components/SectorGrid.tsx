import type { SectorSummary, SectorOutliers } from "../types/database";
import Header from "./Header";
import SectorCard from "./SectorCard";
import "./SectorGrid.css";

interface RefreshProgress {
  current: number;
  total: number;
  phase: string;
}

interface SectorGridProps {
  sectors: SectorSummary[];
  outliersBySector: Map<string, SectorOutliers>;
  refreshingSectors: Set<string>;
  anyRefreshing: boolean;
  onSectorRefresh: (symbol: string) => void;
  refreshing: boolean;
  lastRefresh: Date | null;
  onRefresh: () => void;
  progress: RefreshProgress | null;
}

function SectorGrid({ sectors, outliersBySector, refreshingSectors, anyRefreshing, onSectorRefresh, refreshing, lastRefresh, onRefresh, progress }: SectorGridProps) {
  return (
    <div className="sector-grid">
      <Header
        refreshing={refreshing}
        lastRefresh={lastRefresh}
        onRefresh={onRefresh}
        progress={progress}
      />
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
