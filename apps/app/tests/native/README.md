# Native smoke fixture

`mock_rpc.py` is a loopback-only deterministic JSON-RPC server. It never forwards
requests to Solana. It models two empty legacy SPL accounts of one fungible mint,
a public all-zero fixture seed, signature confirmation and exact close metadata.
It validates token program/source/owner destinations, simulation and preflight.

```sh
# apps/app, terminal 1
python3 tests/native/mock_rpc.py --confirmation-delay=2

# apps/app, terminal 2; choose a fresh isolated journal path for each reset fixture
CARGO_BUILD_JOBS=2 npm run desktop:build -- --debug
DOCK_FLINTS_RPC_URL=http://127.0.0.1:18999 \
DOCK_FLINTS_JOURNAL_PATH=/tmp/flints-native-test/signatures.json \
../../target/debug/dock-flints-app
```

Connect using base64 `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=`. This is public
test input, not a user key. The UI should show two accounts and one selectable
mint. Uncheck it: close count zero, CTA disabled. Check it, click RECOVER SOL once, observe
actual RPC waiting stages, then a confirmed **+0.00406856 SOL** net change for two
close transactions. Gross returned account lamports are 0.00407856 SOL; the
10,000-lamport difference represents the two fixture fees. Inspect fixture state
at `http://127.0.0.1:18999` and the nonsensitive journal to confirm exactly two
signatures. Restarting the fixture resets its chain; do not reuse it to test new
transactions with an old journal/blockhash pair.

The optional delay is in mock RPC replies, not UI progress. Debug inspector can be
enabled with loopback `WEBKIT_INSPECTOR_SERVER` / `WEBKIT_INSPECTOR_HTTP_SERVER` for
native DOM/screenshot/IPC inspection. Do not enable a production remote inspector.
The standard release build is still `npm run desktop:build`.

This smoke test verifies native React/Tauri/core wiring for empty-account close.
Rust tests separately cover swap/burn, failed/uncertain signatures, stale plans,
DAS and provider failures. Neither this fixture nor devnet validates mainnet
Jupiter routing. See `../../docs/desktop-verification.md` for executed results.

The unified WSOL unwrap + nonempty fungible + classic/pNFT scenario was also executed with a real
local validator through the packaged release React/Tauri app. See
[unified verification](../../../../docs/unified-cleanup-verification.md) for exact
receipts, coverage limits, screenshots and reproduction using `nft_local --prepare-only`.
