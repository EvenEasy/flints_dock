import type { AssetCategory, CategoryItem } from '../../../frontend-contract/categories';
import type { ScanStatus, WalletAnalysis } from '../../../frontend-contract/types';

export type CargoCategory = 'scam' | 'nft' | 'dust' | 'dead_token';

function uniqueItems(items: CategoryItem[]): CategoryItem[] {
  const found = new Map<string, CategoryItem>();
  for (const item of items) {
    const previous = found.get(item.id);
    found.set(item.id, {
      ...(previous ?? item),
      accounts: [...new Set([...(previous?.accounts ?? []), ...item.accounts])],
    });
  }
  return [...found.values()];
}

/** Items are the evidence behind the count; overlapping projections never add holdings. */
function normalizeCategory(category: AssetCategory): AssetCategory {
  const itemsAvailable = Array.isArray(category.items);
  const items = uniqueItems(itemsAvailable ? category.items : []);
  const confirmedEmpty = itemsAvailable && category.count === 0;
  const status =
    category.status.status === 'complete' && items.length === 0 && !confirmedEmpty
      ? ({
          status: 'partial',
          reason: 'The category did not return a confirmed item list/count.',
        } as const)
      : category.status;
  return { ...category, items, count: items.length, status };
}

function nftFallback(analysis: WalletAnalysis): AssetCategory | undefined {
  if (!analysis.nfts && !analysis.cnfts) return undefined;
  const missing: ScanStatus = { status: 'skipped', reason: 'NFT discovery was not requested.' };
  const coverage = {
    nft_classic: analysis.nfts?.classic.status ?? missing,
    nft_core: analysis.nfts?.core.status ?? missing,
    das: analysis.cnfts?.status ?? {
      status: 'unsupported' as const,
      reason: 'Compressed NFT coverage is unavailable.',
    },
  };
  const base = (id: string, name: string, kind: string): CategoryItem => ({
    id,
    name,
    kind,
    mint: null,
    program: null,
    accounts: [],
    rawAmount: null,
    risk: null,
    valuation: 'unpriced',
    valueUsd: null,
    tradability: 'unknown',
    evidence: null,
    checkedAt: '0',
    providerScope: null,
  });
  const items = uniqueItems([
    ...(analysis.nfts?.classic.items ?? []).map((nft) => ({
      ...base(nft.mint, nft.metadata.name ?? nft.mint, 'nft'),
      mint: nft.mint,
      program: 'Solana / Metaplex',
      accounts: nft.tokenAccounts,
      evidence: nft.evidence,
    })),
    ...(analysis.nfts?.core.items ?? [])
      .filter((asset) => asset.owner === analysis.owner)
      .map((asset) => ({
        ...base(asset.address, asset.name, 'core'),
        program: 'MPL Core',
        evidence: 'Core AssetV1 with verified wallet ownership',
      })),
    ...(analysis.cnfts?.items ?? []).map((asset) => ({
      ...base(asset.id, asset.name, 'compressed'),
      program: 'Bubblegum / DAS',
      evidence: 'Owner-verified compressed NFT from DAS',
    })),
  ]);
  const listsAvailable =
    analysis.nfts?.classic.items != null &&
    analysis.nfts?.core.items != null &&
    analysis.cnfts?.items != null;
  const incomplete = Object.entries(coverage).filter(([, status]) => status.status !== 'complete');
  const reason = incomplete.length
    ? incomplete
        .map(([name, status]) => `${name}: ${'reason' in status ? status.reason : ''}`)
        .join('; ')
    : 'NFT discovery did not return all item lists.';
  const status: ScanStatus =
    listsAvailable && incomplete.length === 0
      ? { status: 'complete' }
      : { status: 'partial', reason };
  return {
    items,
    count: items.length,
    status,
    checkedAt: '0',
    source: 'Solana / Metaplex / MPL Core / DAS',
    network: analysis.categories?.network ?? 'unknown',
    reason: status.status === 'complete' ? null : reason,
    coverage,
  };
}

/** Normalize once when IPC resolves, so tiles and details consume the same category result. */
export function normalizeWalletCategories(analysis: WalletAnalysis): WalletAnalysis {
  const categories = Object.fromEntries(
    Object.entries(analysis.categories?.categories ?? {})
      .filter(([, category]) => category != null)
      .map(([key, category]) => [key, normalizeCategory(category)]),
  );
  if (!categories.nft) {
    const nft = nftFallback(analysis);
    if (nft) categories.nft = nft;
  }
  if (!analysis.categories && Object.keys(categories).length === 0) return analysis;
  return {
    ...analysis,
    categories: {
      network: 'unknown',
      dustThresholdUsd: 0,
      providers: {},
      ...analysis.categories,
      categories,
    },
  };
}

/** The numeric field carries only a count or dash; diagnostics remain separate. */
export function presentCategory(category: AssetCategory | undefined) {
  const status = category?.status;
  const reason = category?.reason ?? (status && 'reason' in status ? status.reason : null);
  const coverage = category?.coverage ?? {};
  const count = category?.count;
  const confirmedCount = count != null && Number.isSafeInteger(count) && count >= 0;
  const counter =
    category && category.items.length > 0
      ? String(category.items.length)
      : status?.status === 'complete' && confirmedCount && count === 0
        ? '0'
        : '—';
  const incomplete = status?.status !== 'complete';
  const description = [
    counter === '—'
      ? 'No confirmed count is available.'
      : incomplete
        ? `${counter} verified assets found. This is a lower bound; the check is incomplete.`
        : `${counter} verified assets found. Check complete.`,
    reason,
    ...Object.entries(coverage).map(
      ([name, entry]) =>
        `${name}: ${entry.status}${'reason' in entry ? ` (${entry.reason})` : ''}.`,
    ),
  ]
    .filter(Boolean)
    .join(' ');
  return { counter, status, reason, coverage, description };
}
