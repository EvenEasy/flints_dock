import type {
  AccountSummary, AssetList, CompressedNfts, Mint, Nfts,
  Price, TokenAccount, TokenAsset, UnknownAsset,
} from './assets';

/** The command may resolve with useful partial data; inspect category status before rendering. */
export type ScanStatus =
  | { status: 'complete' }
  | { status: 'partial'; reason: string }
  | { status: 'unsupported'; reason: string }
  | { status: 'failed'; reason: string }
  | { status: 'skipped'; reason: string };

/** Stable command rejection envelope. Scanner/network failures are returned in ScanStatus. */
export interface AppError {
  code: 'invalid_request' | 'invalid_wallet_address' | 'invalid_configuration';
  message: string;
  details: Record<string, string> | null;
}

export interface SelectedCategories {
  balance: boolean;
  tokens: boolean;
  allTokens: boolean;
  nfts: boolean;
  cnfts: boolean;
}

/** Omitted/empty selection means all semantic categories. allTokens stays opt-in. */
export interface CategorySelection extends Partial<SelectedCategories> {
  all?: boolean;
}

/** Addresses only. The desktop API never accepts a keypair, seed, or execution mode. */
export interface AnalyzeWalletRequest {
  walletAddress: string;
  selection?: CategorySelection;
  noPrices?: boolean;
}

export interface NativeBalance {
  lamports: string;
  sol: string;
  price: Price | null;
  valueUsd: number | null;
}

export interface Balance {
  status: ScanStatus;
  value: NativeBalance | null;
}

/** Never coerce exact amounts to Number. Use BigInt for integer arithmetic if needed. */
export interface WalletAnalysis {
  owner: string;
  commitment: string;
  selected: SelectedCategories;
  hasUsableResults: boolean;
  balance: Balance | null;
  tokens: AssetList<TokenAsset> | null;
  allTokens: AssetList<TokenAsset> | null;
  nfts: Nfts | null;
  cnfts: CompressedNfts | null;
  tokenAccounts: TokenAccount[] | null;
  mints: Mint[] | null;
  unknownAssets: UnknownAsset[];
  accountSummary: AccountSummary | null;
  scanners: Record<string, ScanStatus>;
}

