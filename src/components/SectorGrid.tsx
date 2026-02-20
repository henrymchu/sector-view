import { useEffect, useRef, useState } from "react";
import type { SectorSummary, SectorOutliers, UniverseType } from "../types/database";
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
  universe: UniverseType;
  onUniverseChange: (universe: UniverseType) => void;
}

const FLIP_DURATION_MS = 500;
const FLIP_STAGGER_MS = 50;

function SectorGrid({ sectors, outliersBySector, refreshingSectors, anyRefreshing, onSectorRefresh, refreshing, lastRefresh, onRefresh, progress, universe, onUniverseChange }: SectorGridProps) {
  const [isFlipping, setIsFlipping] = useState(false);
  const isFirstRender = useRef(true);
  const flipTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      return;
    }

    setIsFlipping(true);
    if (flipTimeoutRef.current) clearTimeout(flipTimeoutRef.current);

    // Total time = animation duration + max stagger across all cards
    const maxCards = 11; // one per GICS sector
    const totalMs = FLIP_DURATION_MS + maxCards * FLIP_STAGGER_MS;
    flipTimeoutRef.current = setTimeout(() => setIsFlipping(false), totalMs);

    return () => {
      if (flipTimeoutRef.current) clearTimeout(flipTimeoutRef.current);
    };
  }, [universe]);

  return (
    <div className={`sector-grid universe-${universe}`}>
      <Header
        refreshing={refreshing}
        lastRefresh={lastRefresh}
        onRefresh={onRefresh}
        progress={progress}
        universe={universe}
        onUniverseChange={onUniverseChange}
      />
      {sectors.map((sector, index) => (
        <SectorCard
          key={sector.symbol}
          sector={sector}
          outliers={outliersBySector.get(sector.symbol)}
          sectorRefreshing={refreshingSectors.has(sector.symbol)}
          anyRefreshing={anyRefreshing}
          onSectorRefresh={onSectorRefresh}
          isFlipping={isFlipping}
          flipIndex={index}
        />
      ))}
    </div>
  );
}

export default SectorGrid;
