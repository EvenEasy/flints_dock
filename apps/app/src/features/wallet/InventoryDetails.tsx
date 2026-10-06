import { useState } from 'react';
import type { WalletAnalysis } from '../../../frontend-contract/types';
import { CategoryNotice } from '../../shared/ui/Notice';
import { lamportsToSol } from '../../shared/format';

/** Expose backend assessments and diagnostics without authorizing closure. */
export function InventoryDetails({ analysis }: { analysis: WalletAnalysis }) {
  const [expanded, setExpanded] = useState(false);
  const [accountsExpanded, setAccountsExpanded] = useState(false);

  // Defer large record lists until their disclosure is opened.
  return (
    <details
      className="inventory-details"
      onToggle={(event) => setExpanded(event.currentTarget.open)}
    >
      <summary>ACCOUNT INVENTORY & DIAGNOSTICS</summary>
      {expanded && (
        <>
          <dl>
            <div>
              <dt>Owner</dt>
              <dd>{analysis.owner}</dd>
            </div>
            <div>
              <dt>Commitment</dt>
              <dd>{analysis.commitment}</dd>
            </div>
            <div>
              <dt>Token accounts</dt>
              <dd>{analysis.accountSummary?.tokenAccounts ?? 'Unavailable'}</dd>
            </div>
            <div>
              <dt>Account lamports (SOL)</dt>
              <dd>{lamportsToSol(analysis.accountSummary?.tokenAccountLamports)}</dd>
            </div>
            <div>
              <dt>Potential rent (SOL)</dt>
              <dd>
                {lamportsToSol(analysis.accountSummary?.potentiallyReclaimableLamports)} ·
                assessment only
              </dd>
            </div>
            <div>
              <dt>Needs closure review</dt>
              <dd>{analysis.accountSummary?.closureReviewAccounts ?? 'Unavailable'}</dd>
            </div>
          </dl>
          {Object.entries(analysis.scanners).map(([name, status]) => (
            <section key={name}>
              <h2>{name}</h2>
              <p>{status.status}</p>
              <CategoryNotice status={status} />
            </section>
          ))}
          <details onToggle={(event) => setAccountsExpanded(event.currentTarget.open)}>
            <summary>TOKEN ACCOUNT RECORDS</summary>
            {accountsExpanded &&
              (analysis.tokenAccounts === null ? (
                <p>Unavailable</p>
              ) : analysis.tokenAccounts.length === 0 ? (
                <p>No token accounts found.</p>
              ) : (
                analysis.tokenAccounts.map((account) => (
                  <article key={account.address}>
                    <h3>{account.address}</h3>
                    <dl>
                      <div>
                        <dt>Mint</dt>
                        <dd>{account.mint}</dd>
                      </div>
                      <div>
                        <dt>Program / state</dt>
                        <dd>
                          {account.program} / {account.state}
                        </dd>
                      </div>
                      <div>
                        <dt>Raw amount</dt>
                        <dd>{account.rawAmount}</dd>
                      </div>
                      <div>
                        <dt>Decimals</dt>
                        <dd>{account.decimals ?? 'Unknown'}</dd>
                      </div>
                      <div>
                        <dt>Extensions</dt>
                        <dd>{account.extensionTypes.join(', ') || 'None'}</dd>
                      </div>
                      <div>
                        <dt>Closure assessment</dt>
                        <dd>
                          {account.closure.status}
                          {'reason' in account.closure ? `: ${account.closure.reason}` : ''}
                        </dd>
                      </div>
                    </dl>
                  </article>
                ))
              ))}
          </details>
          {analysis.unknownAssets.length > 0 && (
            <section>
              <h2>UNCLASSIFIED ASSETS</h2>
              {analysis.unknownAssets.map((asset) => (
                <article key={asset.address}>
                  <code>{asset.address}</code>
                  <p>{asset.reason}</p>
                </article>
              ))}
            </section>
          )}
        </>
      )}
    </details>
  );
}
