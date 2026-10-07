import { useState } from 'react';
import { previewTokens } from '../preview/designData';
import { media } from '../../shared/assets';
import { Icon } from '../../shared/ui/Icon';
import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { CategoryNotice, Notice } from '../../shared/ui/Notice';
import { decimal, shortAddress, usd, usdUnitPrice } from '../../shared/format';
import type { ScreenProps } from '../../app/ScreenProps';

/** Local keep intent is keyed exclusively by mint. It never calls quote or execution APIs. */
export function TokensScreen({
  preview,
  analysis,
  ignoredMints,
  onToggleMint,
}: ScreenProps & { ignoredMints: ReadonlySet<string>; onToggleMint: (mint: string) => void }) {
  const [demoSelected, setDemoSelected] = useState(
    () => new Set(previewTokens.filter((token) => token.selected).map((token) => token.id)),
  );
  const [page, setPage] = useState(0);
  const [view, setView] = useState<'tokens' | 'allTokens'>('tokens');
  const category = view === 'tokens' ? analysis?.tokens : analysis?.allTokens;
  const tokens = category?.items;

  // Bound rendered rows while retaining the complete backend snapshot.
  const visibleTokens = tokens?.slice(page * 50, (page + 1) * 50);

  return (
    <>
      <ScreenTitle
        subtitle={
          preview
            ? 'Select tokens to swap to SOL via Jupiter'
            : 'Read-only balances. Swap routes have not been checked.'
        }
      >
        {preview ? 'SWAPPABLE TOKENS' : 'TOKEN INVENTORY'}
      </ScreenTitle>
      {!preview && (
        <>
          <Notice tone="amber">
            Jupiter USD prices are estimates, not swap quotes. “Keep” marks local intent only; no
            cleanup is submitted.
          </Notice>
          <div className="view-tabs" aria-label="Token inventory view">
            {(['tokens', 'allTokens'] as const).map((key) => (
              <button
                type="button"
                key={key}
                aria-pressed={view === key}
                onClick={() => {
                  setView(key);
                  setPage(0);
                }}
              >
                {key === 'tokens' ? 'TOKENS' : 'ALL ACCOUNTS'}
              </button>
            ))}
          </div>
          <CategoryNotice status={category?.status} />
          <CategoryNotice status={analysis?.scanners.prices} label="USD PRICES" />
        </>
      )}
      <div
        className="token-table"
        role="table"
        aria-label={preview ? 'Reference swap estimates' : 'Wallet token inventory'}
      >
        <div className="token-table__head" role="row">
          <span role="columnheader">TOKEN</span>
          <span role="columnheader">BALANCE</span>
          <span role="columnheader">{preview ? 'EST. SOL' : 'PRICE / TOKEN'}</span>
          <span role="columnheader" className="sr-only">
            {preview ? 'Swap selection' : 'Keep token'}
          </span>
        </div>
        {preview
          ? previewTokens.map((token) => (
              <div className="token-row" role="row" key={token.id}>
                <div className="token-identity" role="cell">
                  <img src={media(`tokens/token-${token.art}.png`)} alt="" width="36" height="36" />
                  <div>
                    <strong>{token.ticker}</strong>
                    <small>{token.name}</small>
                  </div>
                </div>
                <span role="cell" className="token-balance">
                  {token.balance}
                </span>
                <div role="cell" className="token-quote">
                  <strong>{token.sol}</strong>
                  <small>≈ {token.usd}</small>
                </div>
                <div role="cell">
                  <button
                    type="button"
                    className="switch"
                    role="switch"
                    aria-checked={demoSelected.has(token.id)}
                    aria-label={`Select ${token.ticker} for preview swap`}
                    onClick={() =>
                      setDemoSelected((previous) => {
                        const next = new Set(previous);
                        if (next.has(token.id)) next.delete(token.id);
                        else next.add(token.id);
                        return next;
                      })
                    }
                  >
                    <span />
                  </button>
                </div>
              </div>
            ))
          : visibleTokens?.map((token) => (
              <div
                className="token-row token-row--live"
                role="row"
                key={`${token.program}:${token.mint}:${token.accounts.join(':')}`}
              >
                <div className="token-identity" role="cell">
                  <Icon name="wallet" />
                  <div>
                    <strong>{token.metadata.symbol ?? shortAddress(token.mint)}</strong>
                    <small>{token.metadata.name ?? 'Unlabelled asset'}</small>
                    <small className="mint-label" title={token.mint}>
                      {shortAddress(token.mint)} · {token.kind}
                    </small>
                  </div>
                </div>
                <span
                  role="cell"
                  className="token-balance"
                  title={token.balance ?? token.totalRawAmount}
                >
                  {token.balance === null ? `${token.totalRawAmount} raw` : decimal(token.balance)}
                  {token.valueUsd !== null && (
                    <small className="wallet-usd">{usd(token.valueUsd)}</small>
                  )}
                </span>
                <div
                  role="cell"
                  className="token-quote"
                  title={
                    token.price
                      ? `${token.price.usd} USD per token · ${token.price.source}`
                      : undefined
                  }
                >
                  <strong>{usdUnitPrice(token.price?.usd)}</strong>
                  <small>{token.price ? 'USD / TOKEN' : 'PRICE UNAVAILABLE'}</small>
                </div>
                <div role="cell">
                  <button
                    type="button"
                    className="switch switch--keep"
                    role="switch"
                    aria-checked={ignoredMints.has(token.mint)}
                    aria-label={`Keep ${token.metadata.symbol ?? 'token'} mint ${token.mint}`}
                    onClick={() => onToggleMint(token.mint)}
                  >
                    <span />
                  </button>
                </div>
              </div>
            ))}
      </div>
      {!preview && tokens && tokens.length > 50 && (
        <nav className="inventory-pagination" aria-label="Token inventory pages">
          <button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>
            PREVIOUS
          </button>
          <span>
            Page {page + 1} / {Math.ceil(tokens.length / 50)}
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
      {!preview &&
        (!category ? (
          <p className="empty-state">
            This category was not requested. Rescan and enable the corresponding category.
          </p>
        ) : tokens?.length === 0 ? (
          <p className="empty-state">No token assets found in this category.</p>
        ) : tokens === null ? (
          <p className="empty-state">
            Token inventory is unavailable. This is not an empty wallet.
          </p>
        ) : null)}
      <div className="selection-footer">
        <span>
          {preview
            ? `${demoSelected.size} TOKENS SELECTED`
            : `${ignoredMints.size} MINTS MARKED TO KEEP`}
        </span>
        <strong>{preview ? '≈ 0.0283 SOL' : 'LOCAL ONLY'}</strong>
      </div>
      {preview && (
        <p className="micro-note">
          Reference estimate stays fixed; selections change only the demo view.
        </p>
      )}
      {!preview && ignoredMints.size > 0 && (
        <div className="protected-mints">
          <p className="eyebrow">MARKED MINTS</p>
          {[...ignoredMints].map((mint) => (
            <code key={mint}>{mint}</code>
          ))}
        </div>
      )}
    </>
  );
}
