# Invoice contract

Soroban contract for tokenized receivables. Each invoice is stored as an [`Invoice`](src/lib.rs) record; [`get_metadata`](src/lib.rs) exposes a wallet- and explorer-friendly view without duplicating storage.

## `get_metadata(id: u64) -> InvoiceMetadata`

Returns a structured [`InvoiceMetadata`](src/lib.rs) value derived from the stored invoice.

| Field | Type | Meaning |
|--------|------|--------|
| `name` | `String` | Human-readable title, e.g. `Astera Invoice #1`. |
| `description` | `String` | Same as the on-chain invoice description. |
| `image` | `String` | Placeholder URI for artwork (e.g. SVG); replace with per-invoice URIs when available. |
| `amount` | `i128` | Face value in the smallest unit of the settlement asset (USDC on Stellar: 7 decimals). |
| `debtor` | `String` | Counterparty / debtor label. |
| `due_date` | `u64` | Unix timestamp (ledger seconds) when payment is due. |
| `status` | `InvoiceStatus` | `Pending`, `Funded`, `Paid`, or `Defaulted`. |
| `symbol` | `String` | Short ticker (`INV-{id}`), analogous to fungible-token `symbol` in [SEP-0041](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md). |
| `decimals` | `u32` | `7` — interprets `amount` like USDC on Stellar; aligns with SEP-0041 `decimals`. |

### SEP alignment

- **[SEP-0041](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)** defines Soroban token `name`, `symbol`, and `decimals`. This contract maps them to `name`, `symbol`, and `decimals` on `InvoiceMetadata`. Invoice face value is not a SEP-0041 balance call; `amount` + `decimals` still lets UIs format amounts consistently.
- **SEP-0039** in this repo’s issue text refers to a *queryable* token metadata shape. There is no single Soroban substitute that returns arbitrary JSON; `InvoiceMetadata` is the canonical on-chain schema. Wallets may serialize it to JSON with `name`, `description`, and `image` for display, matching common NFT-style metadata conventions.

### JSON example (off-chain projection)

```json
{
  "name": "Astera Invoice #1",
  "description": "Invoice #001 - Goods delivery",
  "image": "https://astera.io/metadata/invoice/placeholder.svg",
  "amount": "1000000000",
  "debtor": "ACME Corp",
  "due_date": 1735689600,
  "status": "Pending",
  "symbol": "INV-1",
  "decimals": 7
}
```

UIs should treat `amount` as an integer string in the smallest unit and divide by `10^decimals` for display.
