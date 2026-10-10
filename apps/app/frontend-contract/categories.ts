import type { ScanStatus } from './types';
export interface CategoryItem {
  id: string;
  mint: string | null;
  program: string | null;
  name: string;
  kind: string;
  accounts: string[];
  rawAmount: string | null;
  risk: { status: string; source: string; reasons: string[]; checkedAt: string } | null;
  valuation: string;
  valueUsd: number | null;
  tradability: string;
  evidence: string | null;
  checkedAt: string;
  providerScope: string | null;
}
export interface AssetCategory {
  items: CategoryItem[];
  count: number;
  status: ScanStatus;
  checkedAt: string;
  source?: string;
  network?: string;
  reason?: string | null;
  coverage?: Record<string, ScanStatus>;
}
export interface WalletCategories {
  network: string;
  dustThresholdUsd: number;
  categories: Record<string, AssetCategory>;
  providers: Record<string, ScanStatus>;
}
