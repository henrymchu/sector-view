import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import SectorGrid from "../SectorGrid";
import {
  mockSectors,
  mockSector,
  mockOutliers,
  mockOutliersSingle,
} from "../../test/fixtures/sectors";

const defaultProps = {
  sectors: mockSectors,
  outliersBySector: new Map(),
  refreshingSectors: new Set<string>(),
  anyRefreshing: false,
  onSectorRefresh: vi.fn(),
  refreshing: false,
  lastRefresh: null,
  onRefresh: vi.fn(),
  progress: null,
  universe: "sp500" as const,
  onUniverseChange: vi.fn(),
};

describe("SectorGrid", () => {
  // ---- Sector card rendering ----

  describe("sector card rendering", () => {
    it("renders a card for each sector in the array", () => {
      render(<SectorGrid {...defaultProps} />);
      // Each sector name should appear exactly once
      expect(screen.getByText("Information Technology")).toBeInTheDocument();
      expect(screen.getByText("Energy")).toBeInTheDocument();
      expect(screen.getByText("Utilities")).toBeInTheDocument();
      expect(screen.getByText("Financials")).toBeInTheDocument();
    });

    it("renders the correct number of buttons (refresh + universe toggle + sector mini)", () => {
      render(<SectorGrid {...defaultProps} />);
      // Header: 1 refresh + 2 universe toggle buttons; SectorCards: 1 per sector
      const buttons = screen.getAllByRole("button");
      // 1 header refresh + 2 universe toggle + 4 sector mini = 7
      expect(buttons).toHaveLength(mockSectors.length + 3);
    });

    it("renders no sector cards when sectors array is empty", () => {
      render(<SectorGrid {...defaultProps} sectors={[]} />);
      expect(screen.queryByText("Information Technology")).not.toBeInTheDocument();
      expect(screen.queryByText("Energy")).not.toBeInTheDocument();
    });

    it("renders a single sector correctly", () => {
      render(<SectorGrid {...defaultProps} sectors={[mockSector]} />);
      expect(screen.getByText("Information Technology")).toBeInTheDocument();
      expect(screen.queryByText("Energy")).not.toBeInTheDocument();
    });
  });

  // ---- Outlier data propagation ----

  describe("outlier propagation", () => {
    it("passes outliers from map to the matching sector card", () => {
      const outliersBySector = new Map([["XLK", mockOutliers]]);
      render(<SectorGrid {...defaultProps} outliersBySector={outliersBySector} />);
      // The XLK card should show the outlier count
      expect(screen.getByText("2 outliers")).toBeInTheDocument();
      // AAPL is the top outlier
      expect(screen.getByText("AAPL")).toBeInTheDocument();
    });

    it("shows no outlier info for sectors not in the map", () => {
      const outliersBySector = new Map([["XLK", mockOutliers]]);
      render(<SectorGrid {...defaultProps} outliersBySector={outliersBySector} />);
      // Energy (XLE) has no outliers in the map — no outlier count text for it
      expect(screen.queryByText("0 outliers")).not.toBeInTheDocument();
    });

    it("handles multiple sectors with outliers", () => {
      const outliersBySector = new Map([
        ["XLK", mockOutliers],
        ["XLU", mockOutliersSingle],
      ]);
      render(<SectorGrid {...defaultProps} outliersBySector={outliersBySector} />);
      expect(screen.getByText("2 outliers")).toBeInTheDocument();
      expect(screen.getByText("1 outlier")).toBeInTheDocument();
    });
  });

  // ---- Refresh state propagation ----

  describe("refresh state propagation", () => {
    it("disables sector mini refresh buttons when anyRefreshing is true", () => {
      render(<SectorGrid {...defaultProps} anyRefreshing={true} />);
      // Each sector's mini refresh button should be disabled; test a specific one
      expect(
        screen.getByRole("button", { name: "Refresh Information Technology" })
      ).toBeDisabled();
      expect(
        screen.getByRole("button", { name: "Refresh Energy" })
      ).toBeDisabled();
    });

    it("marks the refreshing sector's button aria-label as Refreshing", () => {
      const refreshingSectors = new Set(["XLK"]);
      render(
        <SectorGrid
          {...defaultProps}
          refreshingSectors={refreshingSectors}
          anyRefreshing={true}
        />
      );
      expect(
        screen.getByRole("button", { name: "Refreshing Information Technology" })
      ).toBeInTheDocument();
    });
  });

  // ---- Header integration ----

  describe("header", () => {
    it("renders the app title in the header", () => {
      render(<SectorGrid {...defaultProps} />);
      expect(screen.getByText("GICS Intelligence")).toBeInTheDocument();
    });

    it("renders the global Refresh button", () => {
      render(<SectorGrid {...defaultProps} />);
      expect(
        screen.getByRole("button", { name: "Refresh all data" })
      ).toBeInTheDocument();
    });

    it("shows last refresh time when provided", () => {
      const lastRefresh = new Date("2025-01-15T14:30:00");
      render(<SectorGrid {...defaultProps} lastRefresh={lastRefresh} />);
      expect(screen.getByText(/Updated/)).toBeInTheDocument();
    });

    it("renders universe toggle buttons", () => {
      render(<SectorGrid {...defaultProps} />);
      expect(screen.getByRole("button", { name: "S&P 500" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Russell 2000" })).toBeInTheDocument();
    });

    it("marks the active universe button as pressed", () => {
      render(<SectorGrid {...defaultProps} universe="russell2000" />);
      expect(screen.getByRole("button", { name: "Russell 2000" })).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByRole("button", { name: "S&P 500" })).toHaveAttribute("aria-pressed", "false");
    });

    it("calls onUniverseChange when a universe button is clicked", () => {
      const onUniverseChange = vi.fn();
      render(<SectorGrid {...defaultProps} onUniverseChange={onUniverseChange} />);
      fireEvent.click(screen.getByRole("button", { name: "Russell 2000" }));
      expect(onUniverseChange).toHaveBeenCalledWith("russell2000");
    });
  });

  // ---- Snapshots ----

  describe("snapshots", () => {
    it("matches snapshot with full sector data", () => {
      const { container } = render(<SectorGrid {...defaultProps} />);
      expect(container.firstChild).toMatchSnapshot();
    });

    it("matches snapshot with no sectors", () => {
      const { container } = render(
        <SectorGrid {...defaultProps} sectors={[]} />
      );
      expect(container.firstChild).toMatchSnapshot();
    });

    it("matches snapshot with outliers populated", () => {
      const outliersBySector = new Map([["XLK", mockOutliers]]);
      const { container } = render(
        <SectorGrid {...defaultProps} outliersBySector={outliersBySector} />
      );
      expect(container.firstChild).toMatchSnapshot();
    });
  });
});
