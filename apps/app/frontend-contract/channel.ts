import { Channel } from '@tauri-apps/api/core';
import type { CleanupProgress } from './cleanup';

/** Scoped native channels are optional for browser fixtures; their absence never simulates progress. */
export function progressChannel(onProgress?: (event: CleanupProgress) => void) {
  const bridge: unknown = Reflect.get(window, '__TAURI_INTERNALS__');
  if (
    !onProgress ||
    typeof bridge !== 'object' ||
    bridge === null ||
    typeof Reflect.get(bridge, 'transformCallback') !== 'function'
  )
    return undefined;
  const channel = new Channel<CleanupProgress>();
  channel.onmessage = onProgress;
  return channel;
}
