import { useEffect, useState } from 'react';
import type { RefObject } from 'react';
import { mediaSrc, rasterSrc } from '../assets';
import type { MediaKey } from '../assets';

/** Resolve one source from the actual paint size; border slices scale independently of the panel. */
export function useRasterSource(
  media: MediaKey,
  image: RefObject<HTMLElement | null>,
  border = false,
) {
  const [src, setSrc] = useState(() => rasterSrc(media, border ? 96 : 384));
  useEffect(() => {
    const element = image.current;
    if (!element || typeof ResizeObserver === 'undefined') return;
    const update = () => {
      const box = element.getBoundingClientRect();
      const aspect = media === 'decoration.sabers' ? 3 : media === 'scene.station' ? 2 / 3 : 1;
      const style = getComputedStyle(element);
      const width = border
        ? Math.max(parseFloat(style.borderTopWidth), parseFloat(style.borderLeftWidth)) /
          (parseFloat(style.borderImageSlice) / 100)
        : style.objectFit === 'cover'
          ? Math.max(box.width, box.height * aspect)
          : Math.min(box.width, box.height * aspect);
      if (width > 0) setSrc(rasterSrc(media, Math.ceil(width * devicePixelRatio)));
    };
    const observer = new ResizeObserver(update);
    observer.observe(element);
    const density = matchMedia(`(resolution: ${devicePixelRatio}dppx)`);
    density.addEventListener('change', update);
    window.addEventListener('resize', update);
    update();
    return () => {
      observer.disconnect();
      density.removeEventListener('change', update);
      window.removeEventListener('resize', update);
    };
  }, [media, image, border]);
  return src || mediaSrc(media);
}
