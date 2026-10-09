import { useState } from 'react';
import type { WalletAnalysis } from '../../../frontend-contract/types';
import { Dialog } from '../../shared/ui/Dialog';
import { CategoryNotice } from '../../shared/ui/Notice';
import { decimal, shortAddress, usd, usdUnitPrice } from '../../shared/format';
import { Icon } from '../../shared/ui/Icon';
import { InventoryDetails } from '../wallet/InventoryDetails';

/** Inspect actual snapshot categories and prices; unavailable collections remain distinct from empty ones. */
export function InventoryDialog({
  analysis,
  initialView = 'tokens',
  onClose,
}: {
  analysis: WalletAnalysis | null;
  initialView?: 'tokens' | 'nfts';
  onClose: () => void;
}) {
  const [view, setView] = useState<'tokens' | 'allTokens' | 'nfts' | 'cnfts'>(initialView);
  const [page, setPage] = useState(0);
  const category = view === 'tokens' ? analysis?.tokens : analysis?.allTokens;
  const tokens = category?.items;
  return (
    <Dialog title="ТРЮМИ" onClose={onClose}>
      <div className="view-tabs" aria-label="Token inventory view">
        {(['tokens', 'allTokens', 'nfts', 'cnfts'] as const).map((key) => (
          <button
            key={key}
            type="button"
            aria-pressed={view === key}
            onClick={() => {
              setView(key);
              setPage(0);
            }}
          >
            {
              { tokens: 'TOKENS', allTokens: 'ALL ACCOUNTS', nfts: 'NFT / CORE', cnfts: 'cNFT' }[
                key
              ]
            }
          </button>
        ))}
      </div>
      {(view === 'tokens' || view === 'allTokens') && (
        <>
          <CategoryNotice status={category?.status} />
          <CategoryNotice status={analysis?.scanners.prices} label="USD PRICES" />
          <ul className="inventory-list">
            {tokens?.slice(page * 50, (page + 1) * 50).map((token) => (
              <li key={`${token.program}:${token.mint}`}>
                <Icon name="wallet" />
                <div>
                  <strong>{token.metadata.symbol ?? shortAddress(token.mint)}</strong>
                  <p>{token.metadata.name ?? 'Unlabelled asset'}</p>
                  <code>{token.mint}</code>
                  <small>
                    {token.program} · {token.kind}
                  </small>
                  <p className="inventory-balance">
                    {token.balance === null
                      ? `${token.totalRawAmount} raw`
                      : decimal(token.balance)}
                  </p>
                </div>
                <div className="inventory-price" title={token.price?.source}>
                  <strong>{usdUnitPrice(token.price?.usd)}</strong>
                  <small>{token.price ? 'USD / TOKEN' : 'PRICE UNAVAILABLE'}</small>
                  {token.valueUsd !== null && <p>{usd(token.valueUsd)}</p>}
                </div>
              </li>
            ))}
          </ul>
          {!category ? (
            <p className="empty-state">
              This category was not requested. Rescan and enable the corresponding category.
            </p>
          ) : tokens === null ? (
            <p className="empty-state">
              Token inventory is unavailable. This is not an empty wallet.
            </p>
          ) : tokens?.length === 0 ? (
            <p className="empty-state">No token assets found in this category.</p>
          ) : null}
          {tokens && tokens.length > 50 && (
            <nav className="inventory-pagination" aria-label="Token inventory pages">
              <button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>
                PREVIOUS
              </button>
              <span>
                {page + 1} / {Math.ceil(tokens.length / 50)}
              </span>
              <button
                type="button"
                disabled={(page + 1) * 50 >= tokens.length}
                onClick={() => setPage(page + 1)}
              >
                NEXT
              </button>
            </nav>
          )}
        </>
      )}
      {view === 'nfts' && (
        <>
          <CategoryNotice status={analysis?.nfts?.status} />
          <h3>METAPLEX NFT</h3>
          <CategoryNotice status={analysis?.nfts?.classic.status} />
          {analysis?.nfts?.classic.items === null || !analysis?.nfts ? (
            <p>Classic NFT inventory unavailable.</p>
          ) : analysis.nfts.classic.items.length === 0 ? (
            <p>No classic NFTs found.</p>
          ) : (
            <ul className="collectible-list">
              {analysis.nfts.classic.items.map((nft) => (
                <li key={nft.mint}>
                  <strong>{nft.metadata.name ?? shortAddress(nft.mint)}</strong>
                  <code>{nft.mint}</code>
                  <p>
                    {nft.programmable ? 'Programmable NFT' : 'NFT'} · {nft.evidence}
                  </p>
                </li>
              ))}
            </ul>
          )}
          <h3>MPL CORE</h3>
          <CategoryNotice status={analysis?.nfts?.core.status} />
          {analysis?.nfts?.core.items === null || !analysis?.nfts ? (
            <p>Core inventory unavailable.</p>
          ) : analysis.nfts.core.items.length === 0 ? (
            <p>No Core assets found.</p>
          ) : (
            <ul className="collectible-list">
              {analysis.nfts.core.items.map((asset) => (
                <li key={asset.address}>
                  <strong>{asset.name}</strong>
                  <code>{asset.address}</code>
                  <CategoryNotice status={asset.pluginsStatus} />
                </li>
              ))}
            </ul>
          )}
        </>
      )}
      {view === 'cnfts' && (
        <>
          <CategoryNotice status={analysis?.cnfts?.status} />
          {analysis?.cnfts?.items ? (
            <ul className="collectible-list">
              {analysis.cnfts.items.map((asset) => (
                <li key={asset.id}>
                  <strong>{asset.name}</strong>
                  <code>{asset.id}</code>
                </li>
              ))}
              {analysis.cnfts.items.length === 0 && (
                <li>Стиснених NFT не знайдено в отриманих сторінках.</li>
              )}
            </ul>
          ) : (
            <p>Перелік стиснених NFT недоступний.</p>
          )}
        </>
      )}
      {analysis && <InventoryDetails analysis={analysis} />}
    </Dialog>
  );
}
