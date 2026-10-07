import { useState } from 'react';
import type { FormEvent } from 'react';
import type {
  AnalyzeWalletRequest,
  SelectedCategories,
  WalletConnection,
  WalletSource,
} from '../../../frontend-contract/types';
import { Dialog } from '../../shared/ui/Dialog';
import { ActionButton } from '../../shared/ui/ActionButton';
import { connectLocalWallet, hasDesktopRuntime, readableError } from '../../shared/api/wallet';
import { Notice } from '../../shared/ui/Notice';
import { looksLikeAddress } from '../../shared/format';

const categories: { key: keyof SelectedCategories; label: string }[] = [
  { key: 'balance', label: 'Native SOL' },
  { key: 'tokens', label: 'Tokens' },
  { key: 'allTokens', label: 'All token-account assets' },
  { key: 'nfts', label: 'NFTs / MPL Core' },
  { key: 'cnfts', label: 'Compressed NFT capability' },
];

/** Transfer credentials once to Rust, then scan using the returned public address only. */
export function ConnectWalletDialog({
  onClose,
  onScan,
  onConnected,
  initialAddress = '',
  onPreview,
}: {
  onClose: () => void;
  onScan: (request: AnalyzeWalletRequest) => void;
  onConnected: (connection: WalletConnection) => void;
  initialAddress?: string;
  onPreview: () => void;
}) {
  const [kind, setKind] = useState<WalletSource['kind']>('seed');
  const [address, setAddress] = useState(initialAddress);
  const [seed, setSeed] = useState('');
  const [path, setPath] = useState('');
  const [noPrices, setNoPrices] = useState(false);
  const [selection, setSelection] = useState<SelectedCategories>({
    balance: true,
    tokens: true,
    allTokens: true,
    nfts: true,
    cnfts: true,
  });
  const [submitted, setSubmitted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const valid =
    kind === 'publicKey'
      ? looksLikeAddress(address.trim())
      : kind === 'seed'
        ? /^[A-Za-z0-9+/]{43}=$/.test(seed.trim())
        : path.trim().length > 0;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitted(true);
    if (!valid || busy) return;
    setBusy(true);
    setFailure(null);
    const source: WalletSource =
      kind === 'publicKey'
        ? { kind, address: address.trim() }
        : kind === 'seed'
          ? { kind, base64: seed.trim() }
          : { kind, path: path.trim() };
    try {
      const pending = connectLocalWallet({ source });
      // Do not retain the seed for retry, parent state, storage, snapshots, or subsequent scans.
      setSeed('');
      setSubmitted(false);
      const connection = await pending;
      onConnected(connection);
      onScan({ walletAddress: connection.walletAddress, selection, noPrices });
    } catch (error: unknown) {
      setSeed('');
      setFailure(readableError(error).message);
    } finally {
      setBusy(false);
    }
  }

  // Browser preview never collects wallet secrets or attempts credential IPC.
  if (!hasDesktopRuntime())
    return (
      <Dialog title="DOCK A WALLET" onClose={onClose}>
        <Notice tone="amber" title="DESKTOP APP REQUIRED">
          This window is a browser preview. Local wallet connection and analysis run in the Tauri
          desktop app.
        </Notice>
        <p className="dialog-copy">Stop the dev server with Ctrl+C. In apps/app, run:</p>
        <pre>
          <code>npm run desktop</code>
        </pre>
        <p className="dialog-copy">
          Use the desktop window that opens. No credentials have been sent. You can browse the
          design here without connecting a wallet.
        </p>
        <ActionButton onClick={onPreview}>EXPLORE DESIGN PREVIEW</ActionButton>
      </Dialog>
    );

  return (
    <Dialog title="DOCK A WALLET" onClose={onClose} dismissible={!busy}>
      <p className="dialog-copy">
        Load a local signing identity or choose a public address for read-only analysis. Connecting
        does not authorize transactions.
      </p>
      <form
        onSubmit={(event) => {
          void submit(event);
        }}
        noValidate
      >
        <fieldset disabled={busy}>
          <legend>WALLET SOURCE</legend>
          <div className="source-options">
            {[
              { key: 'seed', label: 'Seed (base64)' },
              { key: 'keypairFile', label: 'File (keypair)' },
              { key: 'publicKey', label: 'Public key (read only)' },
            ].map(({ key, label }) => (
              <label className="checkbox-label" key={key}>
                <input
                  type="radio"
                  name="wallet-source"
                  value={key}
                  checked={kind === key}
                  onChange={() => {
                    setKind(key as WalletSource['kind']);
                    setSeed('');
                    setSubmitted(false);
                    setFailure(null);
                  }}
                />
                {label}
              </label>
            ))}
          </div>
        </fieldset>
        <label className="field-label" htmlFor="wallet-identity">
          {kind === 'seed'
            ? 'SEED BASE64'
            : kind === 'keypairFile'
              ? 'KEYPAIR FILE PATH'
              : 'WALLET PUBLIC KEY'}
        </label>
        <input
          id="wallet-identity"
          className="text-input"
          name="walletIdentity"
          type={kind === 'seed' ? 'password' : 'text'}
          value={kind === 'seed' ? seed : kind === 'keypairFile' ? path : address}
          onChange={(event) => {
            if (kind === 'seed') setSeed(event.target.value);
            else if (kind === 'keypairFile') setPath(event.target.value);
            else setAddress(event.target.value);
          }}
          disabled={busy}
          maxLength={kind === 'keypairFile' ? 4096 : 44}
          autoComplete="off"
          autoCapitalize="none"
          spellCheck={false}
          aria-invalid={submitted && !valid && !busy}
          aria-describedby={submitted && !valid && !busy ? 'identity-error' : 'identity-help'}
          required
          placeholder={
            kind === 'seed'
              ? 'Base64 of 32 raw Ed25519 seed bytes'
              : kind === 'keypairFile'
                ? '/absolute/path/to/id.json'
                : 'Paste a Solana public address'
          }
        />
        {submitted && !valid && !busy ? (
          <p id="identity-error" className="field-error" role="alert">
            {kind === 'seed'
              ? 'Enter standard base64 encoding of exactly 32 seed bytes.'
              : kind === 'keypairFile'
                ? 'Enter the absolute path to your local Solana JSON keypair file.'
                : 'Enter a valid Solana public key (32–44 base58 characters).'}
          </p>
        ) : (
          <p id="identity-help" className="field-help">
            {kind === 'seed'
              ? 'Raw Ed25519 seed, not a mnemonic or 64-byte keypair. Sent once to Rust; the field is cleared after submission.'
              : kind === 'keypairFile'
                ? 'Rust reads the file directly. Its contents never enter the frontend.'
                : 'The local Rust backend queries the configured Solana RPC provider with this public address.'}
          </p>
        )}
        <fieldset disabled={busy}>
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
            disabled={busy}
            onChange={(event) => setNoPrices(!event.target.checked)}
          />
          Include Jupiter USD prices
        </label>
        <p className="field-help">
          Missing prices do not hide your assets. Empty category selection uses the backend
          defaults.
        </p>
        {failure && (
          <Notice tone="red" alert title="CONNECTION FAILED">
            {failure}
          </Notice>
        )}
        {busy && (
          <p role="status" className="field-help">
            Connecting locally…
          </p>
        )}
        <ActionButton type="submit" icon="wallet" disabled={busy}>
          {busy ? 'CONNECTING…' : 'SCAN WALLET'}
        </ActionButton>
      </form>
    </Dialog>
  );
}
