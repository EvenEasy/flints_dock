import type { WalletAnalysis, InventoryAsset } from '../../../frontend-contract/types';
import { decimal, shortAddress } from '../../shared/format';

export interface SelectableAsset {
  key: string;
  mint: string;
  name: string;
  quantity: string;
  program: string;
  accounts: string[];
  kind: string;
}

function present(asset: InventoryAsset): SelectableAsset {
  return {
    key: asset.id,
    mint: asset.mint ?? asset.id,
    name: asset.name,
    quantity:
      asset.rawAmount === '0'
        ? 'Empty token account'
        : asset.kind === 'core' || asset.kind === 'compressed' || asset.kind === 'nft'
          ? '1 NFT'
          : asset.balance == null
            ? `${asset.rawAmount ?? '—'} raw`
            : decimal(asset.balance),
    program: asset.program,
    accounts: asset.accounts,
    kind: asset.kind,
  };
}

/** Rust's normalized inventory owns live selection; fallback supports older snapshots and fixtures. */
export function selectableAssets(analysis: WalletAnalysis | null): SelectableAsset[] {
  if (analysis?.inventory?.items) return analysis.inventory.items.map(present);
  if (!analysis) return [];
  const rows = new Map<string, SelectableAsset>();
  const views = [...(analysis.allTokens?.items ?? []), ...(analysis.tokens?.items ?? [])];
  const seen = new Set<string>();
  // Reconstruct exact aggregates from backing accounts, never add overlapping token projections.
  const holdings = new Map<
    string,
    { amount: bigint; decimals: number | null; accounts: string[]; mint: string; program: string }
  >();
  for (const account of analysis.tokenAccounts ?? []) {
    if (seen.has(account.address)) continue;
    seen.add(account.address);
    const id = `${account.mint}:${account.program}`;
    const prior = holdings.get(id);
    holdings.set(id, {
      amount: (prior?.amount ?? 0n) + BigInt(account.rawAmount),
      decimals: prior && prior.decimals !== account.decimals ? null : account.decimals,
      accounts: [...(prior?.accounts ?? []), account.address],
      mint: account.mint,
      program: account.program,
    });
  }
  for (const [id, holding] of holdings) {
    const labels = views.find(
      (token) => token.mint === holding.mint && token.program === holding.program,
    );
    const raw = holding.amount.toString();
    const digits = holding.decimals === null ? null : raw.padStart(holding.decimals + 1, '0');
    const balance =
      digits === null
        ? null
        : holding.decimals === 0
          ? digits
          : `${digits.slice(0, -holding.decimals!)}.${digits.slice(-holding.decimals!).replace(/0+$/, '') || '0'}`;
    rows.set(id, {
      key: holding.mint,
      mint: holding.mint,
      name: labels?.metadata.name ?? labels?.metadata.symbol ?? shortAddress(holding.mint),
      quantity:
        raw === '0'
          ? 'Empty token account'
          : balance === null
            ? `${raw} raw`
            : `${decimal(balance)}${labels?.metadata.symbol ? ` ${labels.metadata.symbol}` : ''}`,
      program: holding.program,
      accounts: holding.accounts,
      kind: labels?.kind ?? 'unknown',
    });
  }
  // A semantic-only snapshot already contains aggregate holdings. Use one view, not their union.
  if (holdings.size === 0)
    for (const token of analysis.tokens?.items ?? analysis.allTokens?.items ?? []) {
      rows.set(`${token.mint}:${token.program}`, {
        key: token.mint,
        mint: token.mint,
        name: token.metadata.name ?? token.metadata.symbol ?? shortAddress(token.mint),
        quantity: token.balance === null ? `${token.totalRawAmount} raw` : decimal(token.balance),
        program: token.program,
        accounts: token.accounts,
        kind: token.kind,
      });
    }
  for (const nft of analysis.nfts?.classic.items ?? []) {
    const existing = [...rows.values()].find((row) => row.mint === nft.mint);
    if (existing) {
      existing.kind = 'nft';
      existing.quantity = '1 NFT';
    } else
      rows.set(nft.mint, {
        key: nft.mint,
        mint: nft.mint,
        name: nft.metadata.name ?? shortAddress(nft.mint),
        quantity: '1 NFT',
        program: 'NFT',
        accounts: nft.tokenAccounts,
        kind: 'nft',
      });
  }
  for (const core of analysis.nfts?.core.items ?? [])
    rows.set(core.address, {
      key: core.address,
      mint: core.address,
      name: core.name,
      quantity: '1 NFT',
      program: 'MPL Core',
      accounts: [],
      kind: 'core',
    });
  for (const cnft of analysis.cnfts?.items ?? [])
    if (!rows.has(cnft.id))
      rows.set(cnft.id, {
        key: cnft.id,
        mint: cnft.id,
        name: cnft.name,
        quantity: '1 NFT',
        program: 'Bubblegum',
        accounts: [],
        kind: 'compressed',
      });
  return [...rows.values()].sort((a, b) => a.key.localeCompare(b.key));
}
