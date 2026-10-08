import type { ReactNode } from 'react';
import { designAsset } from '../assets';
import type { DesignPage } from '../assets';

/** SVG surfaces contain only bevels and lights; all UI text remains in the DOM. */
export function SurfaceFrame({
  page,
  asset,
  className = '',
}: {
  page: DesignPage;
  asset: string;
  className?: string;
}) {
  return (
    <img
      className={`surface-frame ${className}`}
      src={designAsset(page, asset)}
      alt=""
      aria-hidden="true"
    />
  );
}

export function MechanicalPanel({
  page,
  asset,
  className = '',
  children,
}: {
  page: DesignPage;
  asset: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section className={`mechanical-panel ${className}`}>
      <SurfaceFrame page={page} asset={asset} />
      <div className="panel-content">{children}</div>
    </section>
  );
}

/** The emblem, portrait, ring, and hat badge are separate original XML resources. */
export function Brand({ variant = 'welcome' }: { variant?: DesignPage }) {
  const isMain = variant === 'main';
  return (
    <header className={`brand brand--${variant}`}>
      <SurfaceFrame
        page={variant === 'scan' ? 'welcome' : variant}
        asset={isMain ? 'frame_brand' : 'brand_shell'}
      />
      <img
        className="brand-emblem"
        src={designAsset(isMain ? 'main' : 'welcome', isMain ? 'flint_emblem' : 'logo_flint')}
        alt=""
        aria-hidden="true"
      />
      <div className="brand-copy">
        <p>{isMain ? 'FLINT’S STATION' : 'FLINT’S DOCK'}</p>
        <p>SECTOR 9G</p>
      </div>
    </header>
  );
}

export function MascotHero({
  corner = false,
  scanning = false,
  label = false,
}: {
  corner?: boolean;
  scanning?: boolean;
  label?: boolean;
}) {
  return (
    <div className={`mascot-hero ${corner ? 'mascot-hero--corner' : ''}`} aria-hidden="true">
      <img className="hero-ring" src={designAsset('welcome', 'porthole_ring')} alt="" />
      {label && <span className="mascot-label">МАСКОТ</span>}
      <img className="hero-portrait" src={designAsset('welcome', 'raptor_portrait_art')} alt="" />
      <img className="hero-badge" src={designAsset('welcome', 'solana_hat_badge')} alt="" />
      {scanning && <img className="hero-beam" src={designAsset('scan', 'scan_beam_art')} alt="" />}
    </div>
  );
}

/** Recompose the documented scenery without baking any page text into an image. */
export function SceneBackground({ page }: { page: DesignPage }) {
  if (page === 'main')
    return (
      <div className="scene scene--main" aria-hidden="true">
        <img className="scene-full" src={designAsset('main', 'background_space_city')} alt="" />
      </div>
    );
  const cleanup = page === 'cleanup';
  return (
    <div className={`scene scene--${page}`} aria-hidden="true">
      <img className="scene-full" src={designAsset('welcome', 'space_nebula_stars')} alt="" />
      <img
        className="scene-planet"
        src={designAsset(
          cleanup ? 'cleanup' : 'welcome',
          cleanup ? 'cleanup_planet' : 'planet_violet',
        )}
        alt=""
      />
      <img
        className="scene-city"
        src={designAsset(
          cleanup ? 'cleanup' : 'welcome',
          cleanup ? 'cleanup_orbital_city' : 'orbital_city',
        )}
        alt=""
      />
      <img
        className="scene-ships"
        src={designAsset(
          cleanup ? 'cleanup' : 'welcome',
          cleanup ? 'cleanup_ships' : 'ships_fleet',
        )}
        alt=""
      />
      <img
        className="scene-full"
        src={designAsset(
          cleanup ? 'cleanup' : 'welcome',
          cleanup ? 'cleanup_perimeter' : 'perimeter_hull',
        )}
        alt=""
      />
      {!cleanup && (
        <img
          className={`scene-floor ${page === 'success' ? 'scene-floor--success' : ''}`}
          src={designAsset(
            page === 'success' ? 'success' : 'welcome',
            page === 'success' ? 'success_dock_scene' : 'dock_floor_foreground',
          )}
          alt=""
        />
      )}
      {page === 'scan' && (
        <img className="scene-flagship" src={designAsset('scan', 'bottom_flagship')} alt="" />
      )}
    </div>
  );
}

export function TitlePedestal({ page }: { page: 'welcome' | 'main' }) {
  return (
    <MechanicalPanel
      page={page}
      asset={page === 'main' ? 'frame_title_pedestal' : 'title_pedestal_shell'}
      className="title-pedestal"
    >
      <img
        className="crossed-sabers"
        src={designAsset('welcome', 'crossed_sabers_art')}
        alt=""
        aria-hidden="true"
      />
      <p className="eyebrow">ОРБІТАЛЬНИЙ</p>
      <h1 id="screen-heading" tabIndex={-1}>
        ДОК ФЛІНТА
      </h1>
      <p className="tagline">
        <span>ЧИСТИМО ТРЮМИ. ДОБУВАЄМО ЦІННЕ.</span>
      </p>
    </MechanicalPanel>
  );
}
