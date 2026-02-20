import type { UniverseType } from "../types/database";
import "./Header.css";

interface RefreshProgress {
  current: number;
  total: number;
  phase: string;
}

interface HeaderProps {
  refreshing: boolean;
  lastRefresh: Date | null;
  onRefresh: () => void;
  progress?: RefreshProgress | null;
  universe: UniverseType;
  onUniverseChange: (universe: UniverseType) => void;
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

function Header({ refreshing, lastRefresh, onRefresh, progress, universe, onUniverseChange }: HeaderProps) {
  const progressLabel = refreshing && progress
    ? progress.phase === "discovery"
      ? "Discovering stocks..."
      : `Fetching ${progress.current}/${progress.total}...`
    : null;

  const progressPercent = progress && progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  return (
    <div className="header-card">
      <h1 className="app-title">GICS Intelligence</h1>
      {lastRefresh && (
        <span className="last-refresh">
          Updated {formatTime(lastRefresh)}
        </span>
      )}
      <div className="universe-toggle" role="group" aria-label="Select universe">
        <button
          className={`toggle-btn ${universe === "sp500" ? "active" : ""}`}
          onClick={() => onUniverseChange("sp500")}
          aria-pressed={universe === "sp500"}
          disabled={refreshing}
        >
          S&amp;P 500
        </button>
        <button
          className={`toggle-btn ${universe === "russell2000" ? "active" : ""}`}
          onClick={() => onUniverseChange("russell2000")}
          aria-pressed={universe === "russell2000"}
          disabled={refreshing}
        >
          Russell 2000
        </button>
      </div>
      <button
        className={`refresh-btn ${refreshing ? "loading" : ""}`}
        onClick={onRefresh}
        disabled={refreshing}
        aria-label={refreshing ? "Refreshing data" : "Refresh all data"}
      >
        {refreshing ? (
          <>
            <svg className="refresh-icon spin" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 12a9 9 0 1 1-6.219-8.56" />
            </svg>
            {progressLabel ? (
              <span className="refresh-progress-info">
                <span>{progressLabel}</span>
                <div className="progress-bar-container">
                  <div
                    className="progress-bar-fill"
                    style={{ width: `${progressPercent}%` }}
                  />
                </div>
              </span>
            ) : (
              "Refreshing..."
            )}
          </>
        ) : (
          <>
            <svg className="refresh-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 12a9 9 0 0 1-15.36 6.36M3 12a9 9 0 0 1 15.36-6.36" />
              <path d="M21 3v9h-9M3 21v-9h9" />
            </svg>
            Refresh
          </>
        )}
      </button>
    </div>
  );
}

export default Header;
