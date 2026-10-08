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
import { lamportsToSol } from '../../shared/format';
import { Notice } from '../../shared/ui/Notice';

export type CargoCategory = 'scam' | 'nft' | 'dust' | 'dead_token';
const categories = [
  ['scam', 'SCAM', 'scam_skull', 42],
  ['nft', 'NFT', 'nft_coin', 24],
  ['dust', 'DUST', 'dust_crystals', 53],
  ['dead_token', 'DEAD TOKEN', 'dead_bone', 23],
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
  const nftCount = nftKnown
    ? (analysis!.nfts!.classic.items?.length ?? 0) + (analysis!.nfts!.core.items?.length ?? 0)
    : '—';
  const rent = lamportsToSol(analysis?.accountSummary?.potentiallyReclaimableLamports);
  return (
    <div className="page page--main">
      <Brand variant="main" />
      <div className="main-hero">
        <MechanicalPanel page="main" asset="frame_left_panel" className="station-side">
          <p className="side-title">
            FLINT
            <br />
            DOCK 9G
          </p>
          <img src={designAsset('main', 'pirate_skull_crossbones')} alt="" aria-hidden="true" />
          <p className="side-status">
            СТАНЦІЯ
            <br />
            АКТИВНА
          </p>
          <span className="status-led status-led--connected" aria-hidden="true" />
        </MechanicalPanel>
        <MascotHero label />
        <MechanicalPanel page="main" asset="frame_right_panel" className="profit-side">
          <p className="side-title">
            ВЧОРА
            <br />
            СМІТТЯ
          </p>
          <img src={designAsset('main', 'rocket_icon')} alt="" aria-hidden="true" />
          <p className="side-title">
            СЬОГОДНІ
            <br />
            ПРОФІТ
          </p>
        </MechanicalPanel>
      </div>
      <TitlePedestal page="main" />
      {!preview && !analysis?.hasUsableResults && (
        <Notice tone="red" alert>
          No usable asset results. Inspect the category diagnostics or rescan.
        </Notice>
      )}
      <MechanicalPanel page="main" asset="frame_scan_panel" className="scan-summary">
        <h2 className="scan-summary-heading">
          <SurfaceFrame page="main" asset="frame_scan_tab" />
          <img src={designAsset('main', 'scan_radar')} alt="" aria-hidden="true" />
          <span>{preview ? 'СКАНУВАННЯ ВАНТАЖНОГО ВІДСІКУ' : 'АНАЛІЗ ВАНТАЖНОГО ВІДСІКУ'}</span>
        </h2>
        <div className="scan-summary-content">
          <div className="scan-total">
            <p>
              {preview ? (
                <>
                  СКАНУВАННЯ ЗАВЕРШЕНО
                  <br />В ТРЮМАХ ВИЯВЛЕНО
                </>
              ) : analysis?.hasUsableResults ? (
                'АНАЛІЗ ЗАВЕРШЕНО'
              ) : (
                'РЕЗУЛЬТАТ НЕДОСТУПНИЙ'
              )}
            </p>
            <strong>{preview ? 142 : (analysis?.accountSummary?.tokenAccounts ?? '—')}</strong>
            <p>{preview ? 'ОБ’ЄКТА МУСОРА' : 'ТОКЕН-АКАУНТІВ'}</p>
          </div>
          <div className="scan-rent">
            <p>{preview ? 'ДОСТУПНА ЗДОБИЧ (RENT)' : 'ПОТЕНЦІЙНИЙ RENT'}</p>
            <strong>{preview ? '0.052' : rent} SOL</strong>
            <p>{preview ? 'ДОСТУПНА ДО ПОВЕРНЕННЯ' : 'ОЦІНКА, НЕ ПЛАН ОЧИЩЕННЯ'}</p>
          </div>
          <img
            className="orbital-station"
            src={designAsset('main', 'orbital_station')}
            alt=""
            aria-hidden="true"
          />
        </div>
      </MechanicalPanel>
      <section className="cargo-manifest" aria-labelledby="manifest-heading">
        <h2 id="manifest-heading">
          <SurfaceFrame page="main" asset="frame_manifest_tab" />
          <img src={designAsset('main', 'cargo_cube')} alt="" aria-hidden="true" />
          <span>МАНІФЕСТ ВАНТАЖУ</span>
        </h2>
        <div className="category-grid">
          {categories.map(([key, label, art, sample]) => (
            <button
              className={`category-card category-card--${key}`}
              key={key}
              type="button"
              onClick={() => onInspect(key)}
              aria-label={`${label}: ${preview ? sample : key === 'nft' ? nftCount : 'класифікація недоступна'}`}
            >
              <SurfaceFrame page="main" asset={`frame_category_${key}`} />
              <img
                className="category-art"
                src={designAsset('main', art)}
                alt=""
                aria-hidden="true"
              />
              <span>{label}</span>
              <strong>{preview ? sample : key === 'nft' ? nftCount : '—'}</strong>
            </button>
          ))}
        </div>
      </section>
      <RecoverButton page="main" onClick={onCleanup} />
    </div>
  );
}
