import { useState } from 'react';
import type { FormEvent } from 'react';
import type { AnalyzeWalletRequest, SelectedCategories } from '../../../frontend-contract/types';
import { Dialog } from '../../shared/ui/Dialog';
import { ActionButton } from '../../shared/ui/ActionButton';
import { looksLikeAddress } from '../../shared/format';

const categories: { key: keyof SelectedCategories; label: string }[] = [
  { key: 'balance', label: 'Native SOL' },
  { key: 'tokens', label: 'Tokens' },
  { key: 'allTokens', label: 'All token-account assets' },
  { key: 'nfts', label: 'NFTs / MPL Core' },
  { key: 'cnfts', label: 'Compressed NFT capability' },
];

/** No keypair, seed, API key, or signing material is collected or persisted in the frontend. */
export function ConnectWalletDialog({
  onClose,
  onScan,
  initialAddress = '',
}: {
  onClose: () => void;
  onScan: (request: AnalyzeWalletRequest) => void;
  initialAddress?: string;
}) {
  const [address, setAddress] = useState(initialAddress);
  const [noPrices, setNoPrices] = useState(true);
  const [selection, setSelection] = useState<SelectedCategories>({
    balance: true,
    tokens: true,
    allTokens: true,
    nfts: true,
    cnfts: true,
  });
  const [submitted, setSubmitted] = useState(false);
  const valid = looksLikeAddress(address.trim());

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitted(true);
    if (!valid) return;
    onScan({ walletAddress: address.trim(), selection, noPrices });
  }

  return (
    <Dialog title="DOCK A WALLET" onClose={onClose}>
      <p className="dialog-copy">
        Read-only analysis. Enter a public Solana address. Never enter a private key or seed phrase.
      </p>
      <form onSubmit={submit} noValidate>
        <label className="field-label" htmlFor="wallet-address">
          WALLET PUBLIC KEY
        </label>
        <input
          id="wallet-address"
          name="walletAddress"
          className="text-input"
          value={address}
          onChange={(event) => setAddress(event.target.value)}
          maxLength={44}
          autoComplete="off"
          autoCapitalize="none"
          spellCheck={false}
          aria-invalid={submitted && !valid}
          aria-describedby={submitted && !valid ? 'address-error' : 'address-help'}
          required
          placeholder="Paste a Solana public address"
        />
        {submitted && !valid ? (
          <p id="address-error" className="field-error" role="alert">
            Enter a valid Solana public key (32–44 base58 characters).
          </p>
        ) : (
          <p id="address-help" className="field-help">
            Your address is sent only to the desktop backend.
          </p>
        )}
        <fieldset>
          <legend>SCAN CATEGORIES</legend>
          <div className="category-options">
            {categories.map(({ key, label }) => (
              <label key={key} className="checkbox-label">
                <input
                  type="checkbox"
                  checked={selection[key]}
                  onChange={(event) => setSelection({ ...selection, [key]: event.target.checked })}
                />
                {label}
              </label>
            ))}
          </div>
        </fieldset>
        <label className="checkbox-label price-option">
          <input
            type="checkbox"
            checked={!noPrices}
            onChange={(event) => setNoPrices(!event.target.checked)}
          />
          Include optional Jupiter USD prices
        </label>
        <p className="field-help">
          Prices use the backend’s configuration. Unavailable prices do not hide your assets. An
          empty category selection uses the backend defaults.
        </p>
        <ActionButton type="submit" icon="wallet">
          SCAN WALLET
        </ActionButton>
      </form>
    </Dialog>
  );
}
