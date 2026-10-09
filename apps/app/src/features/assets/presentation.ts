import type { WalletAnalysis } from '../../../frontend-contract/types';
import { decimal, shortAddress } from '../../shared/format';

export interface SelectableAsset {
  key: string;
  mint: string;
  name: string;
  quantity: string;
  program: string;
}

/** Merge available inventory views by mint without manufacturing valuations or tradability. */
export function selectableAssets(analysis: WalletAnalysis | null): SelectableAsset[] {
  const byMint = new Map<string, SelectableAsset>();
  // allTokens includes account assets omitted by the semantic fungible-token view.
  for (const token of [...(analysis?.tokens?.items ?? []), ...(analysis?.allTokens?.items ?? [])]) {
    // Prefer the backend's aggregated semantic holding over a later per-account projection.
    if (byMint.has(token.mint)) continue;
    byMint.set(token.mint, {
      key: token.mint,
      mint: token.mint,
      name: token.metadata.name ?? token.metadata.symbol ?? shortAddress(token.mint),
      quantity: `${token.balance === null ? `${token.totalRawAmount} raw` : decimal(token.balance)}${token.metadata.symbol ? ` ${token.metadata.symbol}` : ''}`,
      program: token.program,
    });
  }
  for (const nft of analysis?.nfts?.classic.items ?? []) {
    if (!byMint.has(nft.mint))
      byMint.set(nft.mint, {
        key: nft.mint,
        mint: nft.mint,
        name: nft.metadata.name ?? shortAddress(nft.mint),
        quantity: '1 NFT',
        program: 'NFT',
      });
  }
  for (const account of analysis?.tokenAccounts ?? []) {
    if (!byMint.has(account.mint))
      byMint.set(account.mint, {
        key: account.mint,
        mint: account.mint,
        name: shortAddress(account.mint),
        quantity: account.rawAmount === '0' ? 'Empty token account' : `${account.rawAmount} raw`,
        program: account.program,
      });
  }
  return [...byMint.values()];
}
