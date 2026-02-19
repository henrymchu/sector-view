export interface Sector {
  id: number;
  name: string;
  symbol: string;
}

export interface Stock {
  id: number;
  symbol: string;
  name: string;
  sector_id: number | null;
}
