import { expect, it } from 'vitest';
import { wallet } from '../../test/fixtures';
import { selectableAssets } from './presentation';

it('CanonicalInventoryPreservesCoreAndCompressedIdsAndAllBackingAccounts', () => {
  const model = wallet();
  model.inventory = {
    status: { status: 'partial', reason: 'DAS partial' },
    items: [
      {
        id: 'holding',
        mint: 'mint',
        program: 'SPL',
        kind: 'fungible',
        name: 'Token',
        accounts: ['one', 'two'],
        rawAmount: '9007199254740993',
        decimals: 0,
        balance: '9007199254740993',
        valueUsd: null,
        evidence: 'mint',
      },
      {
        id: 'core-id',
        mint: null,
        program: 'Core',
        kind: 'core',
        name: 'Core',
        accounts: [],
        rawAmount: null,
        decimals: null,
        balance: null,
        valueUsd: null,
        evidence: 'owner',
      },
      {
        id: 'compressed-id',
        mint: null,
        program: 'Bubblegum',
        kind: 'compressed',
        name: 'Compressed',
        accounts: [],
        rawAmount: null,
        decimals: null,
        balance: null,
        valueUsd: null,
        evidence: 'DAS',
      },
    ],
  };
  const assets = selectableAssets(model);
  expect(assets.map((a) => a.key)).toEqual(['holding', 'core-id', 'compressed-id']);
  expect(assets[0]?.quantity).toBe('9,007,199,254,740,993');
  expect(assets[0]?.accounts).toEqual(['one', 'two']);
});

it('OverlappingViewsAndRepeatedAccountsCannotDoubleCountOrDropBackingBalances', () => {
  const model = wallet();
  const source = {
    address: 'one',
    mint: model.tokens!.items![0]!.mint,
    program: 'legacy' as const,
    programId: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
    owner: model.owner,
    rawAmount: '1',
    decimals: 0,
    lamports: '2039280',
    dataLen: '165',
    state: 'Initialized',
    delegate: null,
    delegatedAmount: '0',
    closeAuthority: null,
    isNative: false,
    nativeReserveLamports: null,
    extensionTypes: [],
    closure: { status: 'potentiallyReclaimable' as const },
  };
  model.tokenAccounts = [
    { ...source, address: 'one', rawAmount: '9007199254740993', decimals: 0 },
    { ...source, address: 'two', rawAmount: '7', decimals: 0 },
    { ...source, address: 'one', rawAmount: '9007199254740993', decimals: 0 },
  ];
  model.allTokens = model.tokens;
  const assets = selectableAssets(model);
  expect(assets.find((a) => a.mint === source.mint)?.quantity).toMatch(/^9,007,199,254,741,000/);
  expect(assets.find((a) => a.mint === source.mint)?.accounts).toEqual(['one', 'two']);
});

it('AnEmptyAccountOfAnNftMintDoesNotDisplayAnOwnedNft', () => {
  const model = wallet();
  model.inventory = {
    status: { status: 'complete' },
    items: [
      {
        id: 'nft-mint:SPL',
        mint: 'nft-mint',
        program: 'SPL',
        kind: 'nft',
        name: 'NFT mint',
        accounts: ['empty-account'],
        rawAmount: '0',
        decimals: 0,
        balance: '0',
        valueUsd: null,
        evidence: 'Verified NFT mint; empty backing account',
      },
    ],
  };
  expect(selectableAssets(model)[0]?.quantity).toBe('Empty token account');
  expect(selectableAssets(model)[0]?.accounts).toEqual(['empty-account']);
});
