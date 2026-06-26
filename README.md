# PayalaRemit

Instant, low-cost USDC remittance settlement and anchor cash-out confirmation, built on Stellar Soroban.

## Problem

Rosa, a Filipino domestic worker in Dubai, sends $300/month to her mother in
Cebu via a remittance counter that charges 6% in fees and takes 2 days to
clear — money her mother needs same-day for her diabetes medication.

## Solution

Rosa converts AED to USDC and sends it directly to her mother's Stellar
wallet. The USDC leg settles on-chain via this Soroban contract in one
atomic transaction (transfer + ledger record), and her mother's cash-out
at a local PH anchor is confirmed on-chain too — giving both sides a
verifiable, near-instant record instead of trusting a remittance counter's
internal books. The AED→USDC conversion itself uses Stellar's built-in DEX
path-payment, which is why Stellar (not a generic chain) is essential here:
sub-cent fees, ~5 second finality, and native multi-asset trustlines.

## Timeline

Built for a bootcamp/hackathon sprint (1 contract, 1 frontend, ~3–5 days):
- Day 1: Contract scaffolding + `send_remittance`
- Day 2: `confirm_cash_out` + history index + tests
- Day 3: Frontend wiring (Freighter wallet + testnet anchor simulation)
- Day 4–5: Polish, demo script, deploy to testnet

## Stellar Features Used

- XLM / USDC transfers
- Built-in DEX (path payments, off-contract, for the AED→USDC leg)
- Trustlines
- Soroban smart contracts (this repo — settlement ledger + anchor confirmation)
- Local anchor integration (simulated PH cash-out authority)

## Vision and Purpose

OFW remittances are one of the largest cross-border payment corridors in
the world, and the fees extracted by legacy counters are a direct, recurring
tax on low-income families. PayalaRemit demonstrates that the settlement
and last-mile cash-out confirmation can both live on a single, auditable,
low-cost rail — a pattern that generalizes to any remittance corridor with
a willing local anchor.

## Prerequisites

- Rust (`rustup`) with the `wasm32-unknown-unknown` target
- Stellar CLI (`stellar` / `soroban` CLI) — version 21.x or later

```bash
rustup target add wasm32-unknown-unknown
```

## Build

```bash
soroban contract build
```

Output:

## Deployed Contract

| Field | Value |
|-------|-------|
| Contract ID | `CD4NFUPEYOCLFEFEB2GMLBITYSGCOU6ZG4YUF5KQW5UZVKCACJVCA7JM` |
| Network | testnet |
| Explorer | [View on stellar.expert](https://stellar.expert/explorer/testnet/contract/CD4NFUPEYOCLFEFEB2GMLBITYSGCOU6ZG4YUF5KQW5UZVKCACJVCA7JM) |
| Deploy Tx | [View transaction](https://stellar.expert/explorer/testnet/tx/b7e03462a4d8a780e6091d3851c1e6bf88b48b454d3308bf3e45c8e8f690fc2b) |
| Deployed | 2026-06-26 07:05:11 UTC |
| Wallet | freighter (`GAMV…Z5UU`) |
