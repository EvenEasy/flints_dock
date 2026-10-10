import type { WalletAnalysis } from '../../../frontend-contract/types';
import {
  Brand,
  MascotHero,
  MechanicalPanel,
  SurfaceFrame,
  TitlePedestal,
} from '../../shared/ui/Design';
import { RecoverButton } from '../../shared/ui/ActionButton';
import { designAsset } from '../../shared/assets';
import { RasterArt } from '../../shared/ui/RasterArt';
import type { MediaKey } from '../../shared/assets';
import { ExactAmount } from '../../shared/ui/ExactAmount';
import { lamportsToSol } from '../../shared/format';
import { Notice } from '../../shared/ui/Notice';

export type CargoCategory = 'scam' | 'nft' | 'dust' | 'dead_token';
const categories = [
  ['scam', 'SCAM', 42],
  ['nft', 'NFT', 24],
  ['dust', 'DUST', 53],
  ['dead_token', 'DEAD TOKEN', 23],
] as const;

/** Show observed inventory and rent assessments without deriving risk or routes from USD prices. */
export function MainScreen({
  preview,
  analysis,
  onCleanup,
  onInspect,
}: {
  preview: boolean;
  analysis: WalletAnalysis | null;
  onCleanup: () => void;
  onInspect: (category: CargoCategory) => void;
}) {
  const nftKnown =
    analysis?.nfts && (analysis.nfts.classic.items !== null || analysis.nfts.core.items !== null);
  const observedNfts = nftKnown
    ? (analysis!.nfts!.classic.items?.length ?? 0) + (analysis!.nfts!.core.items?.length ?? 0)
    : 0;
  function categoryDisplay(key: CargoCategory): string {
    const result = analysis?.categories?.categories[key];
    if (result?.status.status === 'complete') return String(result.items.length);
    if (result?.items.length) return `${result.items.length} · Partial`;
    if (key === 'nft' && observedNfts > 0) return `${observedNfts} · Partial`;
    return result?.status.status === 'failed' ? 'Unavailable' : 'Not checked';
  }
  const rent = lamportsToSol(analysis?.accountSummary?.potentiallyReclaimableLamports);
  return (
    <div className="page page--main">
      <Brand name="station" />
      <div className="main-hero">
        <MechanicalPanel className="station-side">
          <p className="side-title">
            FLINT
            <br />
            DOCK 9G
          </p>
          <img src={designAsset('main', 'pirate_skull_crossbones')} alt="" aria-hidden="true" />
          <p className="side-status">
            STATION
            <br />
            ACTIVE
          </p>
          <span className="status-led status-led--connected" aria-hidden="true" />
        </MechanicalPanel>
        <MascotHero variant="compact" label />
        <MechanicalPanel className="profit-side">
          <p className="side-title">
            YESTERDAY
            <br />
            JUNK
          </p>
          <img src={designAsset('main', 'rocket_icon')} alt="" aria-hidden="true" />
          <p className="side-title">
            TODAY
            <br />
            PROFIT
          </p>
        </MechanicalPanel>
      </div>
      <TitlePedestal />
      {!preview && !analysis?.hasUsableResults && (
        <Notice tone="red" alert>
          No usable asset results. Inspect the category diagnostics or rescan.
        </Notice>
      )}
      <MechanicalPanel tone="cyan" inset="compact" className="scan-summary">
        <h2 className="scan-summary-heading">
          <SurfaceFrame />
          <img src={designAsset('main', 'scan_radar')} alt="" aria-hidden="true" />
          <span>{preview ? 'CARGO HOLD SCAN' : 'CARGO HOLD ANALYSIS'}</span>
        </h2>
        <div className="scan-summary-content">
          <div className="scan-total">
            <p>
              {preview ? (
                <>
                  SCAN COMPLETE
                  <br />
                  FOUND IN THE HOLDS
                </>
              ) : analysis?.hasUsableResults ? (
                'ANALYSIS COMPLETE'
              ) : (
                'RESULT UNAVAILABLE'
              )}
            </p>
            <strong className="type-numeric type-numeric--metric">
              {preview ? 142 : (analysis?.accountSummary?.tokenAccounts ?? '—')}
            </strong>
            <p>{preview ? 'UNWANTED ASSETS' : 'TOKEN ACCOUNTS'}</p>
          </div>
          <div className="scan-rent">
            <p>{preview ? 'RECLAIMABLE RENT' : 'POTENTIAL RENT'}</p>
            <strong>
              <ExactAmount amount={preview ? '0.052' : rent} />
            </strong>
            <p>{preview ? 'AVAILABLE TO RECLAIM' : 'ESTIMATE · NOT A CLEANUP PLAN'}</p>
          </div>
          <RasterArt
            media="decoration.station"
            className="orbital-station"
            alt=""
            aria-hidden="true"
          />
        </div>
      </MechanicalPanel>
      <section className="cargo-manifest" aria-labelledby="manifest-heading">
        <h2 id="manifest-heading">
          <SurfaceFrame />
          <img src={designAsset('main', 'cargo_cube')} alt="" aria-hidden="true" />
          <span>CARGO MANIFEST</span>
        </h2>
        <div className="category-grid">
          {categories.map(([key, label, sample]) => (
            <button
              className={`category-card category-card--${key}`}
              key={key}
              type="button"
              onClick={() => onInspect(key)}
              aria-label={`${label}: ${preview ? sample : categoryDisplay(key)}`}
              title={
                !preview && analysis?.categories?.categories[key]?.status.status !== 'complete'
                  ? 'Check incomplete or unavailable'
                  : key === 'scam'
                    ? 'Flagged as suspicious by the provider'
                    : undefined
              }
            >
              <SurfaceFrame
                tone={
                  key === 'nft'
                    ? 'gold'
                    : key === 'dust'
                      ? 'mint'
                      : key === 'dead_token'
                        ? 'red'
                        : 'violet'
                }
              />
              <RasterArt
                className="category-art"
                media={`category.${key}` as MediaKey}
                alt=""
                aria-hidden="true"
              />
              <span className="category-label type-caption">{label}</span>
              <strong className="type-numeric type-numeric--count">
                {preview ? (
                  sample
                ) : (
                  <span
                    className={
                      categoryDisplay(key).includes('checked') ||
                      categoryDisplay(key).includes('Partial') ||
                      categoryDisplay(key) === 'Unavailable'
                        ? 'type-caption'
                        : undefined
                    }
                  >
                    {categoryDisplay(key)}
                  </span>
                )}
              </strong>
            </button>
          ))}
        </div>
      </section>
      <RecoverButton onClick={onCleanup} />
    </div>
  );
}
