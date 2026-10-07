import { invoke } from '@tauri-apps/api/core';
import type { ConnectWalletRequest, WalletConnection } from './types';

/** Credentials enter the local backend once; only public connection data returns. */
export function connectWallet(request: ConnectWalletRequest): Promise<WalletConnection> {
  return invoke<WalletConnection>('connect_wallet', { request });
}

/** Drop the backend signer without changing any blockchain state. */
export function disconnectWallet(): Promise<void> {
  return invoke<void>('disconnect_wallet');
}
