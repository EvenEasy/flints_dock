import type {
  AccountSummary,
  AssetList,
  CompressedNfts,
  Mint,
  Nfts,
  Price,
  TokenAccount,
  TokenAsset,
  UnknownAsset,
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
  code:
    | 'invalid_request'
    | 'invalid_wallet_address'
    | 'invalid_configuration'
    | 'invalid_wallet_identity'
    | 'cleanup_rejected';
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

/** Read-only analysis accepts only a public address; identity and cleanup have separate commands. */
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
  categories?: import('./categories').WalletCategories | null;
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

/** Exactly one identity source; seed is a raw 32-byte Ed25519 seed, not a mnemonic. */
export type WalletSource =
  | { kind: 'publicKey'; address: string }
  | { kind: 'seed'; base64: string }
  | { kind: 'keypairFile'; path: string };

/** Local connection does not authorize transactions. */
export interface ConnectWalletRequest {
  source: WalletSource;
}

/** Public information only; the connected keypair remains in Rust memory. */
export interface WalletConnection {
  sessionId?: string;
  walletAddress: string;
  sourceKind: WalletSource['kind'];
  canSign: boolean;
}
