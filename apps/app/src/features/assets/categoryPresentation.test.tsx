import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import type { AssetCategory, CategoryItem } from '../../../frontend-contract/categories';
import type { ScanStatus } from '../../../frontend-contract/types';
import { address, wallet } from '../../test/fixtures';
import { MainScreen } from '../wallet/MainScreen';
import { CategoryDialog } from './CategoryDialog';
import { normalizeWalletCategories, presentCategory } from './categoryPresentation';

function item(id: string): CategoryItem {
  return {
    id,
    mint: id,
    program: 'SPL',
    name: `Verified ${id}`,
    kind: 'fungible',
    accounts: ['account-one', 'account-two'],
    rawAmount: '100',
    risk: null,
    valuation: 'priced',
    valueUsd: 0.001,
    tradability: 'unknown',
    evidence: 'Confirmed by fixture provider',
    checkedAt: '123',
    providerScope: 'fixture only',
  };
}
function category(status: ScanStatus, items: CategoryItem[] = []): AssetCategory {
  return { status, items, count: items.length, checkedAt: '123' };
}

describe('Category counters', () => {
  it('uses each canonical result for both the numeric tile and its details', async () => {
    const user = userEvent.setup();
    const keys = ['scam', 'nft', 'dust', 'dead_token'] as const;
    const model = normalizeWalletCategories(
      wallet({
        categories: {
          network: 'fixture',
          dustThresholdUsd: 0.01,
          providers: {},
          categories: Object.fromEntries(
            keys.map((key, index) => [
              key,
              category(
                { status: 'complete' },
                Array.from({ length: index + 1 }, (_, row) => item(`${key}-${row}`)),
              ),
            ]),
          ),
        },
      }),
    );
    let selected = '';
    const main = render(
      <MainScreen
        preview={false}
        analysis={model}
        onCleanup={() => {}}
        onInspect={(key) => {
          selected = key;
        }}
      />,
    );
    for (const [index, key] of keys.entries()) {
      const label = key === 'dead_token' ? 'DEAD TOKEN' : key.toUpperCase();
      const tile = screen.getByRole('button', { name: `${label}: ${index + 1}` });
      expect(tile.querySelector('.type-numeric--count')).toHaveTextContent(String(index + 1));
      expect(tile.querySelector('.type-numeric--count .type-caption')).toBeNull();
      await user.click(tile);
      expect(selected).toBe(key);
      const detail = render(
        <CategoryDialog
          name={key}
          category={model.categories!.categories[key]}
          onClose={() => {}}
        />,
      );
      const dialog = screen.getByRole('dialog');
      expect(within(dialog).getAllByRole('listitem')).toHaveLength(index + 1);
      for (const asset of model.categories!.categories[key]!.items)
        expect(within(dialog).getByText(asset.name)).toBeVisible();
      detail.unmount();
    }
    expect(
      [...main.container.querySelectorAll('.type-numeric--count')].map((node) => node.textContent),
    ).toEqual(['1', '2', '3', '4']);
  });

  it('keeps a confirmed zero distinct from missing/null/undefined counts', () => {
    expect(presentCategory(category({ status: 'complete' })).counter).toBe('0');
    for (const count of [null, undefined]) {
      const invalid = { ...category({ status: 'complete' }), count } as unknown as AssetCategory;
      expect(presentCategory(invalid).counter).toBe('—');
      const model = normalizeWalletCategories(
        wallet({
          categories: {
            network: 'fixture',
            dustThresholdUsd: 0.01,
            providers: {},
            categories: { dust: invalid },
          },
        }),
      );
      expect(presentCategory(model.categories!.categories.dust).counter).toBe('—');
    }
  });

  it.each(['partial', 'failed', 'unsupported', 'skipped'] as const)(
    '%s only counts retained confirmed items, never an unknown empty result',
    (status) => {
      const unknown = category({ status, reason: `${status} provider` });
      expect(presentCategory(unknown).counter).toBe('—');
      const found = presentCategory({ ...unknown, items: [item('one'), item('two')], count: 2 });
      expect(found.counter).toBe('2');
      expect(found.description).toContain('lower bound');
      expect(found.reason).toBe(`${status} provider`);
      expect(found.counter).not.toMatch(/Partial|\+/);
    },
  );

  it('has no sample fallback for initial/missing scans', () => {
    expect(presentCategory(undefined).counter).toBe('—');
    const view = render(
      <MainScreen preview={false} analysis={null} onCleanup={() => {}} onInspect={() => {}} />,
    );
    expect(
      [...view.container.querySelectorAll('.type-numeric--count')].map((node) => node.textContent),
    ).toEqual(['—', '—', '—', '—']);
  });

  it('deduplicates IDs and merges backing accounts before either view renders', () => {
    const repeated = [
      { ...item('same-holding'), accounts: ['one'] },
      { ...item('same-holding'), accounts: ['two', 'one'] },
    ];
    const model = normalizeWalletCategories(
      wallet({
        categories: {
          network: 'fixture',
          dustThresholdUsd: 0.01,
          providers: {},
          categories: { dust: category({ status: 'complete' }, repeated) },
        },
      }),
    );
    const result = model.categories!.categories.dust!;
    expect(result.count).toBe(1);
    expect(result.items[0]!.accounts).toEqual(['one', 'two']);
    expect(presentCategory(result).counter).toBe('1');
  });
});

describe('Legacy NFT normalization', () => {
  function nftWallet() {
    const model = wallet();
    model.nfts!.classic.items = [
      {
        mint: 'classic-id',
        tokenAccounts: ['owned-account'],
        metadata: {
          name: 'Classic NFT',
          symbol: null,
          uri: null,
          imageUri: null,
          tokenStandard: 'NonFungible',
          source: 'Metaplex',
          collection: null,
        },
        programmable: false,
        edition: false,
        evidence: 'Verified standard and positive owned account',
      },
    ];
    model.nfts!.core.items = [
      {
        address: 'core-id',
        owner: address,
        name: 'Core NFT',
        uri: '',
        lamports: '1',
        dataLen: '1',
        updateAuthority: address,
        collection: null,
        pluginsStatus: { status: 'complete' },
      },
    ];
    return model;
  }
  it('retains classic/Core findings without DAS, including the same list in details', () => {
    const model = nftWallet();
    model.nfts!.classic.items!.push(model.nfts!.classic.items![0]!);
    model.nfts!.core.items!.push(model.nfts!.core.items![0]!);
    const result = normalizeWalletCategories(model).categories!.categories.nft!;
    expect(result.count).toBe(2);
    expect(result.items.map((asset) => asset.id)).toEqual(['classic-id', 'core-id']);
    expect(presentCategory(result).counter).toBe('2');
    expect(result.status.status).toBe('partial');
    render(<CategoryDialog name="nft" category={result} onClose={() => {}} />);
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByText('Classic NFT')).toBeVisible();
    expect(screen.getByText('Core NFT')).toBeVisible();
    expect(screen.getByText(/lower bound/)).toBeVisible();
  });
  it('includes compressed NFT IDs and deduplicates overlapping discovery', () => {
    const model = nftWallet();
    model.cnfts = {
      status: { status: 'complete' },
      items: [
        { id: 'classic-id', name: 'Duplicate' },
        { id: 'compressed-id', name: 'Compressed NFT' },
        { id: 'compressed-id', name: 'Duplicate compressed' },
      ],
    };
    const result = normalizeWalletCategories(model).categories!.categories.nft!;
    expect(result.count).toBe(3);
    expect(result.status.status).toBe('complete');
  });
  it('does not replace authoritative category results with independent NFT totals', () => {
    const model = nftWallet();
    model.categories = {
      network: 'fixture',
      dustThresholdUsd: 0.01,
      providers: {},
      categories: { nft: category({ status: 'complete' }, [item('canonical-id')]) },
    };
    const result = normalizeWalletCategories(model).categories!.categories.nft!;
    expect(result.items.map((asset) => asset.id)).toEqual(['canonical-id']);
    expect(presentCategory(result).counter).toBe('1');
  });
  it('zero classic/Core is unknown until compressed coverage is complete', () => {
    const model = wallet();
    expect(
      presentCategory(normalizeWalletCategories(model).categories!.categories.nft).counter,
    ).toBe('—');
    model.cnfts = { status: { status: 'complete' }, items: [] };
    expect(
      presentCategory(normalizeWalletCategories(model).categories!.categories.nft).counter,
    ).toBe('0');
  });
});
