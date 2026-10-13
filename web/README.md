# Trellis web

Marketing site and owner dashboard for the `family-vault` Solana program. Next.js (App Router) + Tailwind, talking to the program through the Anchor client.

This is a Devnet prototype. The program is not audited; do not use it with real funds.

## Run

```shell
npm install
npm run dev      # http://localhost:3000
npm run build    # static export of the marketing pages and the dashboard shell
```

## Configuration

`src/config/solana.ts` is the single place for chain settings. Override with env vars:

- `NEXT_PUBLIC_SOLANA_RPC` (default: Devnet public RPC)
- `NEXT_PUBLIC_PROGRAM_ID` (default: the program ID in `programs/family-vault/src/lib.rs`)
- `NEXT_PUBLIC_USDC_MINT` (default: Circle Devnet USDC)

## The IDL

`src/idl/family_vault.json` (and the generated `family_vault.ts` beside it) must match `programs/family-vault`. Regenerate after any program change with `anchor build`, then copy `target/idl/family_vault.json` here.

## Dashboard scope

The dashboard is the vault owner's view: create a personal vault, deposit USDC, add beneficiaries and guardians, and see pending requests. Approving a withdrawal is done by a guardian from their own device, not here.

## License

GNU Affero General Public License v3.0 only (AGPL-3.0-only). See [LICENSE](../LICENSE).
