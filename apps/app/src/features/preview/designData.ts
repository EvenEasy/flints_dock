/** Literal XML samples belong exclusively to preview; these keys are never sent to IPC. */
export const previewAssets = [
  {
    key: 'preview:usdc',
    name: 'USD Coin',
    quantity: '144 USDC',
    value: '1.2000 SOL',
    art: 'usdc_coin',
    selected: false,
    dead: false,
  },
  {
    key: 'preview:flint_014',
    name: 'Flint #014',
    quantity: '1 NFT',
    value: '0.8000 SOL',
    art: 'flint_nft_thumbnail',
    selected: false,
    dead: false,
  },
  {
    key: 'preview:bonk',
    name: 'Bonk',
    quantity: '2 000 000 BONK',
    value: '0.3000 SOL',
    art: 'bonk_coin',
    selected: true,
    dead: false,
  },
  {
    key: 'preview:wif',
    name: 'dogwifhat',
    quantity: '10 WIF',
    value: '0.1000 SOL',
    art: 'wif_coin',
    selected: true,
    dead: false,
  },
  {
    key: 'preview:old',
    name: 'Old Token',
    quantity: '1 000 OLD',
    value: '0 SOL',
    art: 'old_token_coin',
    selected: true,
    dead: true,
  },
  {
    key: 'preview:lost_cargo_007',
    name: 'Lost Cargo #007',
    quantity: '1 NFT',
    value: '0 SOL',
    art: 'lost_cargo_thumbnail',
    selected: true,
    dead: true,
  },
] as const;
export const previewExcluded = previewAssets
  .filter((asset) => !asset.selected)
  .map((asset) => asset.key);
