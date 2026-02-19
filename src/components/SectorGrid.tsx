import SectorCard, { type Sector } from "./SectorCard";
import "./SectorGrid.css";

const SECTORS: Sector[] = [
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

function SectorGrid() {
  return (
    <div className="sector-grid">
      {SECTORS.map((sector) => (
        <SectorCard key={sector.symbol} sector={sector} />
      ))}
    </div>
  );
}

export default SectorGrid;
