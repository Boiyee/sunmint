# Deploying SUNMINT

About 20 minutes. You need [Foundry](https://getfoundry.sh), the [Fly.io CLI](https://fly.io/docs/flyctl/install/) and a Vercel account.

## 1. Wallets

```sh
cast wallet new   # → VERIFIER_KEY  (co-signs, and is the demo producer)
cast wallet new   # → COMPANY_KEY   (demo buyer)
```

Fund both with Base Sepolia ETH from a faucet (e.g. the Coinbase Developer Platform faucet or Alchemy's). The verifier needs the most: it sends a mint and a list transaction per certificate, about 150 a day. 0.05 ETH is plenty.

## 2. Contracts

```sh
cd contracts
export BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
export PRIVATE_KEY=$VERIFIER_KEY                         # deployer can be the verifier
export VERIFIER_ADDRESS=$(cast wallet address $VERIFIER_KEY)
export METADATA_URI="https://sunmint-verifier.fly.dev/v1/certs/{id}"
forge script script/Deploy.s.sol --rpc-url base_sepolia --broadcast
# optional, with BASESCAN_API_KEY set: add --verify
```

Copy the three printed addresses.

## 3. Register the demo meters

From `node/`, with the env set (see `node/.env.example`):

```sh
export RPC_URL=https://sepolia.base.org CERTS_ADDRESS=0x… MARKET_ADDRESS=0x… USD_ADDRESS=0x… VERIFIER_KEY=0x… COMPANY_KEY=0x…
cargo run --release -p sunmint-verifier -- setup
```

This registers the six meters (each signs its own registration), approves the market, and funds the company with test USD.

## 4. Verifier + meters on Fly.io

```sh
cd node
fly launch --no-deploy --copy-config --name sunmint-verifier
fly volumes create sunmint_data --size 1 --region lhr
fly secrets set RPC_URL=https://sepolia.base.org CERTS_ADDRESS=0x… MARKET_ADDRESS=0x… USD_ADDRESS=0x… VERIFIER_KEY=0x… COMPANY_KEY=0x…
fly deploy
fly logs     # watch the 24 h backfill mint
```

If you pick another app name, update `METADATA_URI` and the web `VITE_API_URL`.

## 5. Web on Vercel

Import the repo, set **Root Directory** to `web`, framework **Vite**, and add the env var:

```
VITE_API_URL=https://sunmint-verifier.fly.dev
```

Deploy. The header chip switches from **DRY RUN** to **BASE SEPOLIA** once the verifier is live, and every tx hash links to Basescan.
