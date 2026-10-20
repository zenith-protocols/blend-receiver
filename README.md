# Blend receiver

Soroban receiver for the Blend emitter handover. The receiver itself is its
token, using OZ's default `FungibleToken` implementation with
`ContractType = Base` and seven decimals. The intended owner is YBX SC.

```text
__constructor(admin, emitter, v1_lp_token, blnd)
```

Integrations are immutable, stored independently under the instance Symbol
keys `Emitter`, `LPToken` and `BLNDTkn`.
Refunds use the current OZ owner directly.
The owner supplies the BLND forwarding destination on each call.

The constructor and five receiver actions check instance/code TTL using
Blend's policy: renew below `30 * 17280` ledgers to `31 * 17280` ledgers.
Token and ownership methods use OZ's default implementations. Instance
storage holds the integration addresses, owner, token metadata and total
supply together; individual balances are persistent entries and allowances
are temporary entries. Writing storage does not automatically renew its TTL.

| Function | Behavior |
| --- | --- |
| `acquire_backstop()` | Owner only. Transfers the owner's full old LP balance to the receiver and queues `(receiver, receiver)` atomically. |
| `mint(to, amount)` | Owner only, and only while the emitter identifies the receiver as its current backstop. Uses OZ's default amount validation and token accounting. Repeated minting is allowed. |
| `forward_emissions(to)` | Owner only. Transfers all held BLND to the supplied destination. |
| `refund()` | Owner only, while the receiver is current backstop. Returns all held old LP to the current owner. |
| `drop(address)` | Owner only. Requests the emitter's one-time 50M BLND drop for one recipient. |

OZ's default token methods and two-step ownership transfer/renunciation
methods remain available. The only receiver-specific error is `900`,
`NotCurrentBackstop`; inherited errors retain their existing codes.

After `acquire_backstop()`, wait the emitter's 31 days and call its public
`swap_backstop()`. While the receiver is current, SC can refund its old LP,
use the optional drop, forward BLND, and mint internal tokens to the intended
successor. For the outgoing handover:

1. SC calls `receiver.mint(successor, amount)` and the emitter's
   `queue_swap_backstop(successor, successor_token)` in one transaction,
   through a batching contract such as Creit Tech's
   [Stellar Router](https://github.com/Creit-Tech/Stellar-Router-Contract) v0,
   `CBZV3HBP672BV7FF3ZILVT4CNPW3N5V2WTJ2LAGOAYW5R7L2D5SLUDFZ`.
2. Wait 31 days and call `swap_backstop()`.
3. SC calls `receiver.forward_emissions(v21_backstop)` to send the final BLND distribution
   to v2.1, then optionally calls `renounce_ownership()`.

The emitter enforces queue size checks and delay. Minting stops once the
receiver is no longer its current backstop. Renunciation disables all
owner-only receiver functions, including forwarding.

## Local verification

```sh
make test
make check
```

`make test` builds the current receiver into `wasm/blend_receiver.wasm`, then
runs the `test-suites` crate. `make check` builds it before formatting and
workspace Clippy checks. There are no separate native or ignored smoke tests.
Stellar CLI 25.2.0+ is required for OZ's SDK specification-shaking feature.
The SDK requirement is `25.3.0`; all OZ packages are pinned to `=0.7.0`.

All tests live in `test-suites/tests/test_receiver.rs`, with shared setup in
`test-suites/src/test_fixture.rs`. Clients come from `contractimport!` and all
contracts run as WASM in a local Soroban environment; BLND and USDC use the
built-in Stellar asset contracts. No network or keys are required.

The v2 and v2.1 stacks each deploy the official audited v2 backstop, pool
factory and lending pool. v2 uses legacy Comet v1.0.0; v2.1 uses fixed Comet
v1.1.0. Real LP shares are deposited into each backstop and each pool enters
its reward zone. The lending pools have no reserves or oracle activity: the
suite exercises backstop migration and receiver behavior, not lending.

The lifecycle test moves the emitter from v2 to the receiver, waits both
31-day handover delays, refunds the legacy LP for SC to burn directly, drops
50M BLND, forwards emissions, mints receiver tokens to enable the outgoing
swap, and moves the emitter to v2.1 with its fresh LP. It checks final
forwarding, disabled mint/refund after handover, working v2.1 emissions and
optional ownership renunciation. Other tests cover owner authorization,
acquisition rollback, chosen forwarding destinations, ownership transfer,
and default mint amount validation.

All WASMs are under `wasm/`. The receiver artifact is rebuilt locally and
ignored; [wasm/sources.json](wasm/sources.json) pins official dependency
download URLs, sizes and SHA256 hashes. The fixture follows Blend's test
setup with an unlimited execution budget; it is functional coverage, not a
resource-limit benchmark.

## License

The receiver source is [MIT licensed](LICENSE). Bundled third-party WASMs
retain their [upstream licenses and source attribution](wasm/README.md).
