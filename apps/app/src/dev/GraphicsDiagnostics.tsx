import { useEffect } from 'react';
import { RasterArt } from '../shared/ui/RasterArt';
import { mediaSrc } from '../shared/assets';
import { MascotHero, MechanicalPanel, SurfaceFrame } from '../shared/ui/Design';
/** Development-only A/B fixture: identical source bytes, plain img versus production composition. */
export function GraphicsDiagnostics() {
  useEffect(() => {
    let active = true;
    const capture = async () => {
      await document.fonts.ready;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      await Promise.all([...document.images].map((img) => img.decode().catch(() => undefined)));
      const report = {
        viewport: { width: innerWidth, height: innerHeight },
        dpr: devicePixelRatio,
        scale: visualViewport?.scale,
        userAgent: navigator.userAgent,
        images: [...document.images].map((img) => {
          const box = img.getBoundingClientRect();
          const style = getComputedStyle(img);
          return {
            src: img.currentSrc,
            naturalWidth: img.naturalWidth,
            naturalHeight: img.naturalHeight,
            x: box.x,
            y: box.y,
            width: box.width,
            height: box.height,
            transform: style.transform,
            filter: style.filter,
            objectFit: style.objectFit,
          };
        }),
        frames: [...document.querySelectorAll('.surface-frame')].map((frame) => {
          const box = frame.getBoundingClientRect();
          const style = getComputedStyle(frame);
          return {
            width: box.width,
            height: box.height,
            slice: style.borderImageSlice,
            border: style.borderTopWidth,
            source: style.borderImageSource,
          };
        }),
      };
      if (active) Reflect.set(window, '__graphicsReport', report);
    };
    void capture();
    const loaded = () => {
      void capture();
    };
    document.addEventListener('load', loaded, true);
    window.addEventListener('resize', loaded);
    return () => {
      active = false;
      document.removeEventListener('load', loaded, true);
      window.removeEventListener('resize', loaded);
    };
  }, []);
  return (
    <main className="graphics-diagnostics">
      <h1>Raster A/B · development only</h1>
      <div className="graphics-grid">
        <section>
          <h2>Plain image</h2>
          <img src={mediaSrc('hero.raptor')} alt="Plain mascot" width={300} height={300} />
        </section>
        <section>
          <h2>Component</h2>
          <div className="graphics-hero">
            <MascotHero variant="compact" />
          </div>
        </section>
        <section>
          <h2>Alpha / categories</h2>
          {(['category.scam', 'category.nft', 'category.dust', 'category.dead_token'] as const).map(
            (key) => (
              <img key={key} src={mediaSrc(key)} alt={key} width={72} height={72} />
            ),
          )}
        </section>
        <section>
          <h2>Production categories</h2>
          {(['category.scam', 'category.nft', 'category.dust', 'category.dead_token'] as const).map(
            (key) => (
              <button className="category-card" key={key}>
                <SurfaceFrame />
                <RasterArt className="category-art" media={key} alt={key} />
              </button>
            ),
          )}
        </section>
        <section>
          <h2>Plain frame</h2>
          <img src={mediaSrc('frame.violet')} alt="Original frame" width={300} height={300} />
        </section>
        <section>
          <h2>Border slice 26%</h2>
          <MechanicalPanel>
            <p>Wide panel</p>
          </MechanicalPanel>
          <MechanicalPanel>
            <p>Two lines</p>
            <p>Content</p>
          </MechanicalPanel>
        </section>
        <section>
          <h2>Opaque scene</h2>
          <img src={mediaSrc('scene.station')} alt="Scene" width={200} />
        </section>
      </div>
    </main>
  );
}
