> Historical verification from 2026-10-09, before the unified cleanup strategy.
> The policy selector and production test manifest described below are removed.
> For current behavior/results use [unified cleanup verification](unified-cleanup-verification.md).

# Devnet cleanup verification

Read-only capture: **2026-10-09T21:33:52+00:00**. Wallet `9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv`. Actual network was verified by `getGenesisHash`: `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`. No user-wallet transaction was signed or submitted. Counts below describe this snapshot and are not application constants.

## Confirmed causes

- Jupiter Price/Tokens/Swap are mainnet-only. Devnet routing was correctly unavailable, but Auto requires a genuine NoRoute before burn; unavailable routing cannot authorize destruction. A separate ExplicitDiscard policy now supplies deliberate destruction consent instead.
- The UI previously hid the skip reason and conflated holding count, executable account count, valuation and action. These now have separate labels and structured backend reasons.
- Empty closure previously required fungible classification/mint metadata; post-burn supply changes could therefore block close. Empty closure now validates account state, ownership, authority and extensions independently of mint classification.
- Zero decimals without metadata previously stayed Unknown even for multi-unit supplies. Valid multi-unit mints now classify as FungibleAsset unless NFT evidence conflicts; supply=1 ambiguity remains Unknown. All mints in this public capture actually have decimals=6, so this fix is verified in regressions and the owned validator test rather than inferred from this wallet.
- CLI scan also previously initialized mainnet pricing for a devnet RPC when a key was present. CLI pricing now verifies genesis; unverified/nonmainnet scope disables valuation without discarding usable balances. Devnet cleanup no longer initializes Jupiter or depends on irrelevant API-key configuration.
- Devnet test mints have no mainnet risk/price dataset or supported Jupiter route. Unsupported does not mean scam, dust or dead. Classic/Core NFT discovery is independent and complete here; compressed coverage is unavailable without devnet DAS.

## End-to-end account reconciliation

Legacy discovery 26, Token-2022 discovery 0, core snapshot 26, analysis DTO 26, React selectable mint holdings 26, selected plan assets 26, Auto/discard entries 26/26. Address sets and exact raw amounts match. Nothing was silently removed. This wallet has one account per mint; separate regressions cover multiple accounts of one mint, including ignored empty accounts and explicit NONE.

| Read-only policy | Swap | Burn | Empty close | Executable accounts | Skipped |
| --- | ---: | ---: | ---: | ---: | ---: |
| Auto | 0 | 0 | 6 | 6 | 20 |
| ExplicitDiscard preview | 0 | 20 | 6 | 26 | 0 |

Auto skips are `routing_unavailable`; discard authorizes only eligible selected fungibles. Ignored=0, unsupported eligibility=0 and undecodable=0 in this capture. Both public-key previews have `canExecute=false`. An executable-count figure describes eligible operations, not signing authorization.

## Every discovered token account

All rows are legacy SPL (`TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`), initialized, decimals=6, kind=fungible. Verified mint/freeze authorities are absent; effective close authority is the wallet, with no delegate or account/mint extensions. Metadata PDAs were queried successfully and absent; no TokenStandard/edition NFT evidence was found. Edition reads are not applicable to these decimal-6 mints.

The [machine-readable audit](fixtures/devnet-account-audit.json) contains each full account/mint, exact raw amount, supply, authorities, extensions, evidence, eligibility, planned action and reasonCode/reason. The [captured IPC fixture](../apps/app/tests/fixtures/devnet-wallet.json) retains the real analysis and both plans for React E2E replay.

| Account | Mint | Raw amount | Supply | Auto | ExplicitDiscard |
| --- | --- | ---: | ---: | --- | --- |
| `48zXLcu4BPaXow2pTwPiG5MjEfSdQgBSFwwsp17gvWDy` | `BoRqn6ocvLS924cWdyk84gkAeWJhtf5jxx5q9JVmZgXh` | 100000000000 | 100000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `4YfYZZFedUuQdNrU3GiANC1WQa3xzPvWuNX5L5s7YrRr` | `Gi4q3YLv5uqKcc4Jdr37UEkAQLmdvHQ6BqGxMApd6yLK` | 10000000000000 | 10000000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `5e1KwQRu3hWNDaSYWiWWPv11FagCJy9mQ9857NWXT8BG` | `AVLFztQ2EVuTyDjc56KHjnUnyB5wippdAfWAJaoH7nkg` | 10000000000 | 10000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `5mHMn28uhLLpr3Kg8MZ4WjRJQwMJvGMHeDZYML1VTCeh` | `3pVH7hgxnSGVD34BH6RbWPnVvigbJRbx8En87NBixMUq` | 0 | 0 | close: `empty_account` | close: `empty_account` |
| `79QSGTfbHb8KedRKDarbWD1L6qd9P7byu9Swj31tfYP` | `31jvXrawXfyiQrSzZ1MM93BrD3nkfAV5g4pAFaK8cen7` | 1337000000 | 1337000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `8hWkzjzGqyyiMB9r4RP4HoLswqNwSnv8VLzKfMckPGXz` | `144pKjVYj45TRX9nucSgDYRzSDrDzVHqLW93FX8cNEV8` | 0 | 0 | close: `empty_account` | close: `empty_account` |
| `91NBGyzjXGpAMJVTPbjv5vYPSrDh2JGyit9NBMfmTA7B` | `BUmvFqdrKUSDztQmVaWYHmAhhUTKN5CtaiQ3VvHVXwQn` | 100000000 | 100000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `9j9kdKrRXu3Vsj9S1PixfRb1qvyaLxoidz7gZVWoxjGM` | `8EJyoSfx4TKRrKvULNQy5eLzpnD1h2gwD3yRU1jhqQso` | 10000000000000 | 10000000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `A2KWYdLS2j2oSfTJASf5NPP5wFDfrJnHWztuzNYteyEX` | `CAA21Htmj1GLP9XFboMsWXs4HNT9EwpyzyfaDjskLVJZ` | 10000000000 | 10000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `AKziFnTWhAG2R9jhEJnir1Ro6ue5bRXRQTyNpmFQPXQf` | `1325cQDpA5cT1qChQrm3Gad8EiPHwGqp7WFW8j3HFrFi` | 0 | 0 | close: `empty_account` | close: `empty_account` |
| `BWLjCQ7EwFV29EbLzifjY5oqgmJHwB4sGR3d55zJLHe4` | `8i1GvDWWREw6CdnDHsbXnqE2W3qogMKnutQiJdsLRDHp` | 42069000000 | 42069000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `CCdfdTxvtaHuKPfAGzmcKJ1HetmhDuahmS64QDt8DHou` | `6V4RhTQJapM4vk4sL5B3wukXuQQLN4UTijCn5UhJzRiL` | 0 | 0 | close: `empty_account` | close: `empty_account` |
| `CQtuwz94BXd6tyiPVUoMQsh2diYbPtEsDnztVYQL14wS` | `4VPfX1iyzRQknTYX15yMg7P3jHBnB2gmAFcxCQU81ha4` | 5000000000000 | 5000000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `CgUsynH624kV3Gg2BhKUAkR5kqB5hKjjsxnehZhXEghW` | `CQi8qv2xBpnqRz33FszWHQzEC7g5AsL9yL2LQmqb6YpV` | 100000000000 | 100000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `D19PNVXsEFdiM4WJVkmRZEeudVig6hqH3a2K8twAJSjF` | `E861r4hYMLcdCb7CXQ2n2QKSAdXNNMc8DU6JD1BYgBjg` | 0 | 0 | close: `empty_account` | close: `empty_account` |
| `DZA37doCKHPv67HynLzVnsyM36phTX8tvmfj9oogFQSA` | `52WWAL2urufggwtmFKoD7upaZvFsh5mAtJg6CVREbvL1` | 42069000000 | 42069000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `E2QtBtHGYz3KkgsXPFY1HsAZu3JLBoVibkEU9wvSfD5P` | `CL83rkA4MoXu9TaN3kst7BMHEspAshW7Z5LenaUzsyVo` | 0 | 0 | close: `empty_account` | close: `empty_account` |
| `FsDf5rZBpAqanRuxmy2JEM6zrsET1mAS4WwpcF7aqGt4` | `3KTxE2kjiPCsncgpLoMHoyo4gfd8BgZoQLcsaASAbLmh` | 100000000 | 100000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `GWZ4yMRmekQfFKYdRdQVUzVtCoBo3KXMLR2fpcbU5WN2` | `AqqzTM1FqVHnnVpTRrf75RPHwWSeMZB8j7Mgp3YqzPkT` | 420000000 | 420000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `GffqW5ww4dQ64rk2xFR1UYQfrFvCxc1gyhAtG14h1rGB` | `7rD7xxsQjQbkjDEUUUTR6BYyJ19r5LeWwe4CBnGVvoyb` | 420000000 | 420000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `HFx8JHB62ECiCTYKvFeKc8CCKUHtREYVChr81qcNn7Po` | `C4PrtESQaZMf9CFYJmJCnFYRZgUnpaqQd6e2y1cPj3n6` | 5000000000000 | 5000000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `HG7in5oYrY6uEboZg8AsYeMbAZWAhxCqe5GFbeQw3Xq5` | `5Asyp5VyyEGJxopnzS1NkAfk7DeaHX9T7bCJAkUL64pM` | 13000000 | 13000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `HWRSqMCURWrtJXxhDmf6LwARAxXEA7C9TuCAi8nCzn9F` | `Hk1d7uMYam1BU8qHgZHmc1PV3oGXsPog5TLkxAecgvL5` | 999999000000 | 999999000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `JBpTyV2A9RvVZL2UQmb4xQocYsJc4pbUKi9tKPZthmMG` | `Btnyi3NZ9kV1qXsH2F864mr9aWfFehrz7tTw83V8Lfj6` | 42069000000 | 42069000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `S3nHHnjcw9QJ2xyCiFoyweYoe4Wj1ndQxWyd3eMET4D` | `wesnpmyaxpqm7dRbHUrXoKpUrfVUULNYbPM8xEFC3Y1` | 13000000 | 13000000 | skip: `routing_unavailable` | burn: `explicit_discard` |
| `zAYUKMMcPykxzKBroNxrkRrCNaUY8eQA8DjknUSuHLg` | `94e6pYCFcfA793rgYzQ2hYNHgDNHbPq7uuJymqrf54Ka` | 5000000000 | 5000000000 | skip: `routing_unavailable` | burn: `explicit_discard` |

## Category meaning and coverage

- SCAM: only provider-flagged suspicious; devnet risk is unsupported, never inferred from names, prices or authorities.
- DUST: aggregate nonzero mint/program holding with known valuation `0 < USD value <= DOCK_FLINTS_DUST_USD` (default 0.01 USD). No valuation means unknown, not zero/dust.
- DEAD TOKEN: confirmed, provider/time-scoped NoRoute for a nonzero fungible holding. Devnet unsupported routing and ExplicitDiscard are not NoRoute evidence.
- NFT: classic/programmable and Core checks are complete with no items in this capture. Compressed NFT coverage is unsupported because DAS is not configured. Overall NFT is partial, displayed as unavailable/lower bound rather than a definitive zero.

Every category carries items/count/status/source/network/reason/checkedAt, with independent classic/Core/compressed coverage. The optional devnet manifest is clearly TEST DATA and partial; it cannot build a quote, influence cleanup eligibility or authorize a transaction.

## Running devnet cleanup

```sh
export DOCK_FLINTS_RPC_URL=https://api.devnet.solana.com
cd apps/app
npm run desktop
```

Connect a public key to inspect. To execute on your own disposable test wallet, connect its local keypair/seed once, open Cleanup, select **Devnet: discard selected**, uncheck every mint to preserve, and approve the dialog showing the burn-account count and irreversible action. All-unchecked means NONE. Policy/selection changes invalidate prior plans; plans expire after 120 seconds and are bound to session/genesis/exact accounts/amounts. Mainnet retains Auto.

For an explicitly synthetic category demo only:

```sh
export DOCK_FLINTS_DEVNET_TEST_MANIFEST=/absolute/repo/docs/fixtures/devnet-observations.example.json
```

The example IDs were verified in the public devnet inventory, but its risk/value/route labels are deliberately invented test observations. They do not change real balances or NFT evidence, and never supply live devnet swaps. Remove the variable for ordinary analysis. `.env` is not automatically loaded; Rust reads inherited environment variables. Configure `DOCK_FLINTS_DAS_URL` only with a real devnet DAS endpoint reporting the same genesis. Credentials stay in Rust, never VITE variables.

CLI keeps its default behavior; explicit discard additionally requires repeated exact `--account` selection. See [cleanup usage](cleanup.md) and [desktop configuration](../apps/app/README.md).

## Confirmed disposable validator burn/close

A separate owned loopback validator wallet was funded for this test. No public/user keypair was used. The core integration helper permits only localhost/127.0.0.1 RPC, records pending signatures before send, and runs the actual Rust simulation/preflight/confirmation executor. Its source contained 42 units of a zero-decimal, metadata-less multi-unit mint. BurnChecked destroyed all 42, refreshed zero without needing mint metadata, closed to owner, and confirmed account absence.

[Exact report and signatures](fixtures/local-discard-confirmed.json): reclaimed 2,039,280 lamports, burn fee 5,000, close fee 5,000, exact net **2,029,280 lamports = 0.00202928 SOL**; accounting complete, one closed account, zero failures. Burned token value is not added to recovered SOL. These signatures belong to a disposable local ledger and cannot be looked up on devnet explorers.

Reproduce with a new owned fixture (never reuse the already-closed source):

```sh
cargo run -p dock-flints-core --example discard_local -- \
  http://127.0.0.1:8899 /absolute/disposable-wallet.json <OWNED_NONEMPTY_TOKEN_ACCOUNT>
```

A read-only public audit can be repeated without any signer:

```sh
cargo run -p dock-flints-core --example devnet_diagnostics -- \
  https://api.devnet.solana.com 9FCR2PU1jZgCHyjWxzk2BNQHJxszAK24vBFiWmUyRNpv
DOCK_FLINTS_LIVE_READONLY_REPORT=/tmp/devnet-ipc.json \
  cargo test -p dock-flints-app --test ipc live_devnet_readonly_report -- --ignored --nocapture
```

## Verification scope

Rust regressions and real registered Tauri IPC/capability tests use mocked RPC execution for read-only rejection, NONE, ignored/multiple accounts, stale policies/session/network, duplicate requests, exact BurnChecked amounts, confirmation/zero/absence and fee accounting. The optional live IPC test uses actual devnet RPC with a public identity and submits no transactions. Browser E2E replays those captured DTOs and separately verifies signing-flow orchestration with mocks. The actual on-chain execution proof is the owned local-validator integration above; no live Jupiter devnet swap is claimed. The packaged debug desktop was also launched on actual Linux WebKitGTK at
`tauri://localhost`, CSS viewport 1100×687, DPR 2. The real React public-key scan
showed 26 accounts; its Auto and ExplicitDiscard plans matched 6 executable/20
skipped and 26 executable/20 burn/0 skipped respectively. The account list stayed
inside its scroll container, main overflow was zero, the 360×639 app screen was
centered, and the execute button remained disabled for the public identity.
Additional actual native IPC requests confirmed all DTO accounts/amounts and no
signer. No native execute request or user-wallet transaction was made.
[Native public IPC/DOM evidence](fixtures/native-devnet-readonly.json) and
[native screenshot](../apps/app/docs/screenshots/devnet/native-readonly-cleanup.png)
are separate from Chromium/mocked-execution screenshots.

Completed checks on the final code:

- `cargo fmt --all --check`: passed.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: 97 passed; the opt-in live test is ignored by the ordinary suite and was separately run successfully against actual devnet RPC.
- `cargo clippy --workspace --all-targets --all-features`: passed, no warnings.
- `npm run check`: passed (typecheck, lint, 50 unit tests, production frontend build).
- `npm run format:check`: passed.
- `npm run test:e2e`: 84 passed, including 360×800, 390×844, 430×932, centered desktop, phone/reference modes, long lists, exact amounts and actual captured devnet DTO replay.
- `npm run desktop:build -- --debug`: passed with final embedded frontend/core; actual native Tauri/WebKitGTK public-key scan/plan checks passed.
- Actual owned local-validator BurnChecked → confirmed zero → CloseAccount → confirmed absence: passed with complete fee accounting, using a separate disposable wallet.

The actual read-only CLI ExplicitDiscard preview was also run for one selected
public devnet account with deliberately invalid irrelevant Jupiter header
configuration: one Burnable, one close, zero swaps, zero transactions submitted.
[CLI preview](fixtures/devnet-cli-discard-preview.json) verifies the Jupiter-independent
path without any signer.

A separate final native launch with the explicitly configured example manifest
showed `SCAM: 1+`, `DUST: 1+`, `DEAD TOKEN: 1+`, all partial and clearly labelled
TEST DATA. Those are invented observations over actual verified devnet holdings,
not conclusions about their real risk/liquidity/price. Auto still had Burn=0;
public identity `canSign=false`, zero transactions submitted.
[Native test-observation evidence](fixtures/native-devnet-test-observations.json) and
[its disclosure dialog](../apps/app/docs/screenshots/devnet/native-test-data-category.png).

Native execution orchestration is covered through registered IPC with mock RPC;
actual burn/close execution is proven on the owned validator through the same
core executor. No native mainnet/devnet user-wallet execution or live Jupiter
devnet swap is claimed. Without a devnet DAS endpoint, compressed coverage remains
unsupported. No live NFT was found in this wallet; NFT detection is additionally
covered by verified TokenStandard/legacy-edition/Core/DAS fixtures.

Screenshots (Chromium, captured-DTO or mocked execution):
[public inventory at 360×800](../apps/app/docs/screenshots/devnet/devnet-recorded-360.png),
[centered desktop at 1440×900](../apps/app/docs/screenshots/devnet/devnet-recorded-1440.png),
[explicit burn plan](../apps/app/docs/screenshots/devnet/devnet-plan-360.png),
[processing with mocked RPC](../apps/app/docs/screenshots/devnet/devnet-discard-360.png).
