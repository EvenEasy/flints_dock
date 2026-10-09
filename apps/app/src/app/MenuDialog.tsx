import type { WalletAnalysis, WalletConnection } from '../../frontend-contract/types';
import { Dialog } from '../shared/ui/Dialog';
import { CategoryNotice } from '../shared/ui/Notice';
import { decimal, usd, usdUnitPrice } from '../shared/format';
import { screens } from './navigation';
import type { ScreenId } from './navigation';

/** Session controls expose only public connection information; credentials remain in Rust. */
export function MenuDialog({
  preview,
  analysis,
  connection,
  screen,
  onClose,
  onNavigate,
  onConnect,
  onPreview,
  onExitPreview,
  onDisconnect,
}: {
  preview: boolean;
  analysis: WalletAnalysis | null;
  connection: WalletConnection | null;
  screen: ScreenId;
  onClose: () => void;
  onNavigate: (screen: ScreenId) => void;
  onConnect: () => void;
  onPreview: () => void;
  onExitPreview: () => void;
  onDisconnect: () => void;
}) {
  return (
    <Dialog title={preview ? 'DESIGN PREVIEW' : 'WALLET PROFILE'} onClose={onClose}>
      {preview ? (
        <>
          <p className="dialog-copy">
            Five XML reference screens. Sample amounts and stages are not wallet results; no
            transactions are submitted.
          </p>
          <nav className="screen-navigation" aria-label="Station screens">
            {screens.map((item) => (
              <button
                key={item.id}
                type="button"
                aria-current={item.id === screen ? 'page' : undefined}
                onClick={() => onNavigate(item.id)}
              >
                {item.label}
                <span aria-hidden="true">↗</span>
              </button>
            ))}
          </nav>
          <button className="text-button" type="button" onClick={onExitPreview}>
            EXIT PREVIEW
          </button>
        </>
      ) : (
        <>
          {connection && (
            <p className="wallet-capability">
              {connection.canSign ? 'LOCAL SIGNER CONNECTED' : 'PUBLIC ADDRESS · READ ONLY'}
              <code>{connection.walletAddress}</code>
            </p>
          )}
          {analysis?.balance && (
            <section className="wallet-balance" aria-label="Native SOL balance">
              <h3>NATIVE SOL</h3>
              <strong>{decimal(analysis.balance.value?.sol)} SOL</strong>
              <p>
                {analysis.balance.value?.valueUsd == null
                  ? 'USD value unavailable'
                  : `≈ ${usd(analysis.balance.value.valueUsd)}`}
              </p>
              <p title={analysis.balance.value?.price?.source}>
                {analysis.balance.value?.price
                  ? `1 SOL = ${usdUnitPrice(analysis.balance.value.price.usd)}`
                  : 'SOL price unavailable'}
              </p>
              <CategoryNotice status={analysis.balance.status} />
              <CategoryNotice status={analysis.scanners.prices} label="USD PRICES" />
            </section>
          )}
          {connection && (
            <button type="button" className="text-button" onClick={onDisconnect}>
              DISCONNECT WALLET
            </button>
          )}
          <button type="button" className="text-button" onClick={onConnect}>
            CHANGE WALLET / RESCAN
          </button>
          <button type="button" className="text-button" onClick={onPreview}>
            EXPLORE DESIGN PREVIEW
          </button>
        </>
      )}
    </Dialog>
  );
}
