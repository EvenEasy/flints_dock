import { useRef } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { mediaSrc } from '../assets';
import { RasterArt } from './RasterArt';
import { useRasterSource } from './useRasterSource';

export type PanelTone = 'violet' | 'cyan' | 'mint' | 'gold' | 'red';
export type PanelInset = 'standard' | 'compact' | 'spacious' | 'title';

/** The transparent center and fixed corner slices keep metal edges out of the content slot. */
export function SurfaceFrame({
  tone = 'violet',
  className = '',
}: {
  tone?: PanelTone;
  className?: string;
}) {
  const key = tone === 'cyan' || tone === 'mint' ? 'frame.cyan' : 'frame.violet';
  const frame = useRef<HTMLSpanElement>(null);
  const src = useRasterSource(key, frame, true);
  return (
    <span
      ref={frame}
      className={`surface-frame surface-frame--${tone} ${className}`}
      data-media={key}
      style={
        {
          '--frame-art': `url("${src}")`,
        } as CSSProperties
      }
      aria-hidden="true"
    />
  );
}

/** Tone chooses the surface; inset reserves the frame's thickness independently of page layout. */
export function MechanicalPanel({
  tone = 'violet',
  inset = 'standard',
  className = '',
  decoration,
  children,
}: {
  tone?: PanelTone;
  inset?: PanelInset;
  className?: string;
  decoration?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className={`mechanical-panel panel-inset--${inset} ${className}`}>
      <SurfaceFrame tone={tone} />
      {decoration}
      <div className="panel-content">{children}</div>
    </section>
  );
}

/** Both allowed names use identical frame, emblem, text slots, and sector baseline. */
export function Brand({ name = 'dock' }: { name?: 'dock' | 'station' }) {
  return (
    <header className="brand" data-brand={name}>
      <SurfaceFrame />
      <img className="brand-emblem" src={mediaSrc('brand.emblem')} alt="" aria-hidden="true" />
      <div className="brand-copy">
        <p className="type-brand">{name === 'station' ? 'FLINT’S STATION' : 'FLINT’S DOCK'}</p>
        <p className="type-sector">SECTOR 9G</p>
      </div>
    </header>
  );
}

/** The artwork already includes portrait, ring, and hat badge; they must not be overlaid twice. */
export function MascotHero({
  variant = 'large',
  scanning = false,
  label = false,
}: {
  variant?: 'large' | 'compact' | 'corner';
  scanning?: boolean;
  label?: boolean;
}) {
  return (
    <div
      className={`mascot-hero mascot-hero--${variant}`}
      data-hero-variant={variant}
      aria-hidden="true"
    >
      <div className="hero-stage">
        <RasterArt
          media="hero.raptor"
          className="hero-art"
          data-media="hero.raptor"
          alt=""
          width={1254}
          height={1254}
        />
        {label && <span className="mascot-label type-caption">МАСКОТ</span>}
        {scanning && <img className="hero-beam" src={mediaSrc('hero.scanBeam')} alt="" />}
      </div>
    </div>
  );
}

/** One text-free raster scene contains its planet and ships, so no duplicate SVG scenery is added. */
export function SceneBackground() {
  return (
    <div className="scene" aria-hidden="true">
      <RasterArt
        media="scene.station"
        className="scene-art"
        data-media="scene.station"
        alt=""
        width={1024}
        height={1536}
      />
    </div>
  );
}

/** A width-based decorative anchor keeps the swords independent of title wrapping and panel height. */
export function TitlePedestal() {
  return (
    <MechanicalPanel
      inset="title"
      className="title-pedestal"
      decoration={
        <div className="pedestal-anchor" aria-hidden="true">
          <RasterArt
            media="decoration.sabers"
            className="crossed-sabers"
            data-media="decoration.sabers"
            alt=""
            width={2172}
            height={724}
          />
        </div>
      }
    >
      <p className="eyebrow type-section">ОРБІТАЛЬНИЙ</p>
      <h1 id="screen-heading" className="type-display" tabIndex={-1}>
        ДОК ФЛІНТА
      </h1>
      <p className="tagline type-caption">
        <span>ЧИСТИМО ТРЮМИ. ДОБУВАЄМО ЦІННЕ.</span>
      </p>
    </MechanicalPanel>
  );
}
