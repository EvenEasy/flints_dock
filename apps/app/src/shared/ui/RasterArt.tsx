import { useRef } from 'react';
import type { ImgHTMLAttributes } from 'react';
import type { MediaKey } from '../assets';
import { useRasterSource } from './useRasterSource';

/** Prefiltered images preserve the source artwork, alpha and intrinsic aspect ratio. */
export function RasterArt({
  media,
  ...props
}: ImgHTMLAttributes<HTMLImageElement> & { media: MediaKey }) {
  const image = useRef<HTMLImageElement>(null);
  const src = useRasterSource(media, image);
  return <img {...props} ref={image} src={src} />;
}
