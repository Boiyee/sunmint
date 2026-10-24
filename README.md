# SUNMINT

**Every hour of sunlight, minted.** Hourly, on-chain clean-energy certificates from signed small-solar meter readings, weighed by what each hour displaced: the local grid, or a diesel generator.

Built for the IEEE ClimateChain Global Hackathon 2026 (Track 2: Renewable Energy & Grid Management).

## How it works

1. **Meter signs.** Each meter holds its own key and signs a reading every 5 minutes (EIP-712): energy, grid voltage, timestamp, nonce.
2. **Verified.** The verifier checks the signature against the key registered on-chain, rejects reused nonces and stale readings, and checks physics: no output with the sun below the horizon, nothing above the array's rating.
3. **Weighed.** Grid at 0 V all hour *and* a generator registered at the site → diesel factor. Otherwise the country's grid factor. Any doubt → grid.
4. **Minted.** The meter signs an hour claim bound to the Merkle root of its 12 readings. The contract checks the meter's signature **and** the verifier's co-signature, applies the displacement rule itself, and mints exactly one certificate per device-hour (balance in Wh).
5. **Retired.** Companies buy and retire in one transaction. Retired Wh are burned and a public receipt is emitted.

## Repo

| Path | What |
|---|---|
| `contracts/` | Foundry. `SunmintCertificates` (ERC-1155), `SunmintMarket`, `TestUSD` (testnet faucet token), deploy script, tests incl. every attack |
| `node/core` | Shared Rust: EIP-712 types, Merkle tree, sun position, the 5-stage check pipeline, demo sites |
| `node/verifier` | Axum API + SQLite. Verifies readings and claims, co-signs, mints, lists, matches load, retires, runs the Break it attacks |
| `node/simulator` | Six demo meters (Lagos, Nairobi, Jaipur, Manila, Seville, São Paulo) on real sun positions with clouds and outage schedules |
| `web/` | React + Vite PWA: Landing, Daylight, Rooftop, Match, Trace, Proof, Break it |

## Run locally (no chain needed)

```sh
cd node && cargo run -p sunmint-verifier          # dry mode: contract rules mirrored in memory
cd node && cargo run -p sunmint-simulator          # backfills 24 h, then live every 5 min
cd web && npm install && npm run dev               # http://localhost:5173
```

Tests: `cd contracts && forge test` · `cd node && cargo test`

## Deploy to Base Sepolia

See [DEPLOY.md](DEPLOY.md).

## Honest limits

- Testnet only. Demo meters are simulated; their keys are derived from public seeds so anyone can reproduce the Break it attacks. Never use them for real devices.
- Emission factors are demo values (`contracts/script/Deploy.s.sol`, `node/core/src/demo.rs`). Replace with sourced figures.
- A compromised meter could under-report grid voltage. Roadmap: cross-check outages against utility feeds and neighbouring meters.
- In the demo, the verifier wallet also acts as the producer that receives and lists certificates.

MIT
