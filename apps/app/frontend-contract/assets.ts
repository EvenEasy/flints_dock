import type { ScanStatus } from './types';

/** Null items mean unavailable; an empty array means a successful empty inventory. */
export interface AssetList<T> {
  status: ScanStatus;
  items: T[] | null;
}

export type TokenProgram = 'legacy' | 'token2022';
export type AssetKind =
  | 'fungible'
  | 'fungibleAsset'
  | 'nonFungible'
  | 'nonFungibleEdition'
  | 'programmableNonFungible'
  | 'programmableNonFungibleEdition'
  | 'unknown';

export interface Collection {
  address: string;
  verified: boolean;
}

export interface Metadata {
  name: string | null;
  symbol: string | null;
  uri: string | null;
  imageUri: string | null;
  tokenStandard: string | null;
  source: string | null;
  collection: Collection | null;
}

/** USD values are approximate; blockId is an exact decimal integer string. */
export interface Price {
  usd: number;
  source: string;
  blockId: string | null;
  decimals: number;
}

export interface TokenAsset {
  mint: string;
  program: TokenProgram;
  totalRawAmount: string;
  decimals: number | null;
  balance: string | null;
  accounts: string[];
  kind: AssetKind;
  metadata: Metadata;
  price: Price | null;
  valueUsd: number | null;
}

export type ClosureAssessment =
  | { status: 'potentiallyReclaimable' }
  | { status: 'notReclaimable'; reason: string }
  | { status: 'needsReview'; reason: string };

/** This describes an account; it does not authorize wallet mutation. */
export interface TokenAccount {
  address: string;
  mint: string;
  program: TokenProgram;
  programId: string;
  owner: string;
  rawAmount: string;
  decimals: number | null;
  lamports: string;
  dataLen: string;
  state: string;
  delegate: string | null;
  delegatedAmount: string;
  closeAuthority: string | null;
  isNative: boolean;
  nativeReserveLamports: string | null;
  extensionTypes: string[];
  closure: ClosureAssessment;
}

export interface Mint {
  mint: string;
  program: TokenProgram;
  decimals: number;
  supply: string;
  mintAuthority: string | null;
  freezeAuthority: string | null;
  metadata: Metadata;
  extensionTypes: string[];
}

export interface Nft {
  mint: string;
  tokenAccounts: string[];
  metadata: Metadata;
  programmable: boolean;
  edition: boolean;
  evidence: string;
}

export interface CoreAsset {
  address: string;
  owner: string;
  name: string;
  uri: string;
  lamports: string;
  dataLen: string;
  updateAuthority: string;
  collection: Collection | null;
  pluginsStatus: ScanStatus;
}

export interface Nfts {
  status: ScanStatus;
  classic: AssetList<Nft>;
  core: AssetList<CoreAsset>;
}

/** The current core requires a historical owner index and cannot enumerate cNFTs. */
export interface CompressedNfts {
  status: ScanStatus;
  items: null;
}

export interface UnknownAsset {
  address: string;
  programId: string | null;
  lamports: string | null;
  reason: string;
}

export interface AccountSummary {
  tokenAccounts: number;
  emptyTokenAccounts: number;
  tokenAccountLamports: string;
  potentiallyReclaimableLamports: string;
  closureReviewAccounts: number;
}

