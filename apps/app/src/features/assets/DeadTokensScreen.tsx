import { previewDeadTokens } from '../preview/designData';
import { ScreenTitle } from '../../shared/ui/ScreenTitle';
import { Icon } from '../../shared/ui/Icon';
import { Notice } from '../../shared/ui/Notice';
import type { ScreenProps } from '../../app/ScreenProps';

/** Absence of a Price API value is never treated as proof of no route or burn eligibility. */
export function DeadTokensScreen({ preview }: ScreenProps) {
  return (
    <>
      <ScreenTitle
        subtitle={
          preview
            ? 'No liquidity. Safe to burn and remove.'
            : 'Burn eligibility requires a backend route and account assessment.'
        }
      >
        DEAD TOKENS
      </ScreenTitle>
      {!preview ? (
        <Notice tone="amber" title="CLASSIFICATION UNAVAILABLE">
          The read-only scan does not classify dead tokens. Missing USD prices cannot authorize a
          burn. No burn command is registered.
        </Notice>
      ) : (
        <div className="dead-table" role="table" aria-label="Reference dead tokens">
          <div className="dead-table__head" role="row">
            <span role="columnheader">TOKEN</span>
            <span role="columnheader">BALANCE</span>
            <span role="columnheader">ACTION</span>
          </div>
          {previewDeadTokens.map(([ticker, name, balance]) => (
            <div role="row" className="dead-row" key={ticker}>
              <div role="cell" className="token-identity">
                <Icon name="skull" tone="red" />
                <div>
                  <strong>{ticker}</strong>
                  <small>{name}</small>
                </div>
              </div>
              <span role="cell">{balance}</span>
              <div role="cell">
                <button
                  type="button"
                  className="burn-button"
                  disabled
                  title="Reference only; burn is unavailable"
                >
                  BURN
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
      <Notice tone="red" title="BURNING IS PERMANENT">
        These tokens would be irreversibly destroyed and cannot be restored. Burn controls are
        disabled in this frontend.
      </Notice>
    </>
  );
}
