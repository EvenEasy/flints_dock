import type { WalletAnalysis } from '../../frontend-contract/types';
import type { TokenAsset } from '../../frontend-contract/assets';

export const address = 'So11111111111111111111111111111111111111112';
export const secondMint = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

// Exact strings deliberately exceed Number precision to exercise presentation without rounding.
export function token(mint = address): TokenAsset {
  return {
    mint,
    program: 'legacy',
    totalRawAmount: '900719925474099312345',
    decimals: 9,
    balance: '900719925474.099312345',
    accounts: [`account-${mint}`],
    kind: 'fungible',
    metadata: {
      name: 'Same name',
      symbol: 'SAME',
      uri: null,
      imageUri: 'https://untrusted.invalid/token.png',
      tokenStandard: null,
      source: null,
      collection: null,
    },
    price: null,
    valueUsd: null,
  };
}

export function wallet(overrides: Partial<WalletAnalysis> = {}): WalletAnalysis {
  return {
    owner: address,
    commitment: 'confirmed',
    selected: { balance: true, tokens: true, allTokens: true, nfts: true, cnfts: true },
    hasUsableResults: true,
    balance: {
      status: { status: 'complete' },
      value: { lamports: '12345678901', sol: '12.345678901', price: null, valueUsd: null },
    },
    tokens: { status: { status: 'complete' }, items: [token(), token(secondMint)] },
    allTokens: null,
    nfts: {
      status: { status: 'complete' },
      classic: { status: { status: 'complete' }, items: [] },
      core: { status: { status: 'complete' }, items: [] },
    },
    cnfts: {
      status: { status: 'unsupported', reason: 'Historical owner index required.' },
      items: null,
    },
    tokenAccounts: [],
    mints: [],
    unknownAssets: [],
    accountSummary: {
      tokenAccounts: 2,
      emptyTokenAccounts: 1,
      tokenAccountLamports: '2039280',
      potentiallyReclaimableLamports: '2039280',
      closureReviewAccounts: 0,
    },
    scanners: {},
    ...overrides,
  };
}

/** Provider values are fixture quotes; valuation stays backend-owned in every UI test. */
export function pricedWallet(): WalletAnalysis {
  const snapshot = wallet();
  snapshot.balance!.value!.price = {
    usd: 120.5,
    decimals: 9,
    source: 'Jupiter Price V3',
    blockId: '123456789',
  };
  snapshot.balance!.value!.valueUsd = 1487.6543075705;
  const tiny = token('11111111111111111111111111111111');
  tiny.metadata.symbol = 'TINY';
  tiny.balance = '2';
  tiny.totalRawAmount = '2000000000';
  tiny.price = {
    usd: 0.000000000123,
    decimals: 9,
    source: 'Jupiter Price V3',
    blockId: '123456789',
  };
  tiny.valueUsd = 0.000000000246;
  const stable = token(secondMint);
  stable.metadata.symbol = 'USDC';
  stable.decimals = 6;
  stable.balance = '10';
  stable.totalRawAmount = '10000000';
  stable.price = { usd: 1.0022, decimals: 6, source: 'Jupiter Price V3', blockId: '123456789' };
  stable.valueUsd = 10.022;
  snapshot.tokens!.items = [tiny, stable];
  snapshot.allTokens = { status: { status: 'complete' }, items: [tiny, stable] };
  snapshot.scanners.prices = { status: 'complete' };
  return snapshot;
}
