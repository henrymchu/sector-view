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
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

function Header({ refreshing, lastRefresh, onRefresh, progress }: HeaderProps) {
  const progressLabel = refreshing && progress
    ? progress.phase === "discovery"
      ? "Discovering stocks..."
      : `Fetching ${progress.current}/${progress.total}...`
    : null;

  const progressPercent = progress && progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  return (
    <header className="app-header">
      <h1 className="app-title">Sector View</h1>
      <div className="header-actions">
        {lastRefresh && (
          <span className="last-refresh">
            Last updated: {formatTime(lastRefresh)}
          </span>
        )}
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
    </header>
  );
}

export default Header;
