import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import SectorCard from "../SectorCard";
import {
  mockSector,
  mockSectorNoData,
  mockSectorNegativeChange,
  mockSectorMillions,
  mockOutliers,
  mockOutliersSingle,
  mockOutliersEmpty,
} from "../../test/fixtures/sectors";
import type { SectorSummary } from "../../types/database";

const defaultProps = {
  sector: mockSector,
  outliers: undefined,
  sectorRefreshing: false,
  anyRefreshing: false,
  onSectorRefresh: vi.fn(),
};

describe("SectorCard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ---- Identity display ----

  describe("sector identity", () => {
    it("renders sector name", () => {
      render(<SectorCard {...defaultProps} />);
      expect(screen.getByText("Information Technology")).toBeInTheDocument();
    });

    it("renders sector symbol", () => {
      render(<SectorCard {...defaultProps} />);
      expect(screen.getByText("XLK")).toBeInTheDocument();
    });
  });

  // ---- Price change display ----

  describe("price change", () => {
    it("renders positive change with + prefix", () => {
      render(<SectorCard {...defaultProps} />);
      expect(screen.getByText("+2.10%")).toBeInTheDocument();
    });

    it("renders negative change without + prefix", () => {
      render(<SectorCard {...defaultProps} sector={mockSectorNegativeChange} />);
      expect(screen.getByText("-1.50%")).toBeInTheDocument();
    });

    it("shows -- when stock_count is 0", () => {
      render(<SectorCard {...defaultProps} sector={mockSectorNoData} />);
      const changeMetric = screen.getByText("Change").closest(".metric");
      expect(changeMetric).toHaveTextContent("--");
    });

    it("shows -- when avg_change_percent is 0", () => {
      const sector: SectorSummary = { ...mockSector, avg_change_percent: 0 };
      render(<SectorCard {...defaultProps} sector={sector} />);
      const changeMetric = screen.getByText("Change").closest(".metric");
      expect(changeMetric).toHaveTextContent("--");
    });
  });

  // ---- P/E ratio display ----

  describe("P/E ratio", () => {
    it("renders avg P/E ratio formatted to 1 decimal", () => {
      render(<SectorCard {...defaultProps} />);
      expect(screen.getByText("28.5")).toBeInTheDocument();
    });

    it("shows -- when avg_pe_ratio is null", () => {
      render(<SectorCard {...defaultProps} sector={mockSectorNoData} />);
      const peMetric = screen.getByText("Avg P/E").closest(".metric");
      expect(peMetric).toHaveTextContent("--");
    });
  });

  // ---- Market cap formatting ----

  describe("market cap", () => {
    it("formats trillions as $X.XT", () => {
      render(<SectorCard {...defaultProps} />);
      expect(screen.getByText("$12.5T")).toBeInTheDocument();
    });

    it("formats billions as $X.XB", () => {
      render(<SectorCard {...defaultProps} sector={mockSectorNegativeChange} />);
      expect(screen.getByText("$500.0B")).toBeInTheDocument();
    });

    it("formats millions as $XM", () => {
      render(<SectorCard {...defaultProps} sector={mockSectorMillions} />);
      expect(screen.getByText("$750M")).toBeInTheDocument();
    });

    it("shows -- when total_market_cap is null", () => {
      render(<SectorCard {...defaultProps} sector={mockSectorNoData} />);
      const mktCapMetric = screen.getByText("Mkt Cap").closest(".metric");
      expect(mktCapMetric).toHaveTextContent("--");
    });
  });

  // ---- Outlier display ----

  describe("outlier section", () => {
    it("renders outlier count (plural) when multiple outliers exist", () => {
      render(<SectorCard {...defaultProps} outliers={mockOutliers} />);
      expect(screen.getByText("2 outliers")).toBeInTheDocument();
    });

    it("renders singular 'outlier' for exactly one outlier", () => {
      render(<SectorCard {...defaultProps} outliers={mockOutliersSingle} />);
      expect(screen.getByText("1 outlier")).toBeInTheDocument();
    });

    it("renders top outlier symbol", () => {
      render(<SectorCard {...defaultProps} outliers={mockOutliers} />);
      expect(screen.getByText("AAPL")).toBeInTheDocument();
    });

    it("renders top outlier type", () => {
      render(<SectorCard {...defaultProps} outliers={mockOutliers} />);
      expect(screen.getByText("GrowthPremium")).toBeInTheDocument();
    });

    it("renders top outlier composite score", () => {
      render(<SectorCard {...defaultProps} outliers={mockOutliers} />);
      expect(screen.getByText(/2\.1σ/)).toBeInTheDocument();
    });

    it("does not render outlier section when outlier_count is 0", () => {
      render(<SectorCard {...defaultProps} outliers={mockOutliersEmpty} />);
      expect(screen.queryByText(/outlier/i)).not.toBeInTheDocument();
    });

    it("does not render outlier section when outliers prop is undefined", () => {
      render(<SectorCard {...defaultProps} outliers={undefined} />);
      expect(screen.queryByText(/outlier/i)).not.toBeInTheDocument();
    });
  });

  // ---- Refresh button interactions ----

  describe("refresh button", () => {
    it("calls onSectorRefresh with sector symbol on click", () => {
      const onSectorRefresh = vi.fn();
      render(<SectorCard {...defaultProps} onSectorRefresh={onSectorRefresh} />);
      fireEvent.click(screen.getByRole("button"));
      expect(onSectorRefresh).toHaveBeenCalledTimes(1);
      expect(onSectorRefresh).toHaveBeenCalledWith("XLK");
    });

    it("is disabled when anyRefreshing is true", () => {
      render(<SectorCard {...defaultProps} anyRefreshing={true} />);
      expect(screen.getByRole("button")).toBeDisabled();
    });

    it("is enabled when anyRefreshing is false", () => {
      render(<SectorCard {...defaultProps} anyRefreshing={false} />);
      expect(screen.getByRole("button")).not.toBeDisabled();
    });

    it("has aria-label 'Refresh <name>' when not refreshing", () => {
      render(<SectorCard {...defaultProps} sectorRefreshing={false} />);
      expect(screen.getByRole("button")).toHaveAttribute(
        "aria-label",
        "Refresh Information Technology"
      );
    });

    it("has aria-label 'Refreshing <name>' when sectorRefreshing is true", () => {
      render(<SectorCard {...defaultProps} sectorRefreshing={true} />);
      expect(screen.getByRole("button")).toHaveAttribute(
        "aria-label",
        "Refreshing Information Technology"
      );
    });
  });

  // ---- Snapshots ----

  describe("snapshots", () => {
    it("matches snapshot with complete data and outliers", () => {
      const { container } = render(
        <SectorCard {...defaultProps} outliers={mockOutliers} />
      );
      expect(container.firstChild).toMatchSnapshot();
    });

    it("matches snapshot with null/missing data", () => {
      const { container } = render(
        <SectorCard {...defaultProps} sector={mockSectorNoData} />
      );
      expect(container.firstChild).toMatchSnapshot();
    });

    it("matches snapshot while sector is refreshing", () => {
      const { container } = render(
        <SectorCard {...defaultProps} sectorRefreshing={true} anyRefreshing={true} />
      );
      expect(container.firstChild).toMatchSnapshot();
    });
  });
});
