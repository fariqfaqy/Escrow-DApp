# Stellar Escrow DApp

**Stellar Escrow DApp** - Blockchain-Based Decentralized Escrow & Transaction Security System

---

## Project Description

Stellar Escrow DApp is a decentralized smart contract solution built on the Stellar blockchain using Soroban SDK. It provides a secure, trustless platform for managing financial transactions between two parties without requiring a centralized intermediary. The contract ensures that funds are locked safely and only released when predefined conditions are met, eliminating reliance on third-party escrow services.

The system allows users to create escrow agreements, confirm deliveries, and resolve disputes — all directly on the Stellar blockchain. Each escrow is uniquely identified and stored within the contract's persistent storage, ensuring data reliability and full transaction transparency.

> **Note:** This version is a simulation contract. Balances are tracked in contract storage (no real token transfer) and can be used to test all escrow logic directly on Stellar Testnet without token setup.

---

## Project Vision

Our vision is to revolutionize peer-to-peer commerce in the digital age by:

- **Decentralizing Trust**: Replacing traditional escrow agents with immutable, code-enforced smart contracts on the blockchain
- **Ensuring Fairness**: Giving both buyers and sellers equal protection through transparent and pre-agreed escrow rules
- **Guaranteeing Security**: Locking funds in a tamper-proof contract until all conditions are satisfied by both parties
- **Enhancing Accountability**: Providing a permanent, auditable record of every transaction stage that cannot be manipulated
- **Building Trustless Commerce**: Creating a platform where transaction integrity is guaranteed by code, not by company promises

We envision a future where anyone, anywhere, can transact safely without needing to trust a stranger — only the contract.

---

## Key Features

### 1. **Simulation Balance System**

- Top up simulation balance before starting any transaction
- No real token setup required — ideal for testnet exploration
- Per-address balance tracking stored on-chain
- Instant balance checks at any point in the transaction lifecycle

### 2. **Secure Escrow Creation**

- Create escrow agreements with a single function call
- Specify buyer, seller, arbiter, amount, and deal description
- Simulation funds are automatically locked upon creation
- Automated ID generation for unique escrow identification

### 3. **Multi-Stage Transaction Flow**

- Structured status progression: `Pending → InProgress → Completed`
- Seller confirms delivery readiness before funds can be released
- Buyer confirms receipt to trigger fund release to seller
- Every stage change is recorded immutably on the blockchain

### 4. **Built-in Dispute Resolution**

- Either buyer or seller can raise a dispute during `InProgress` stage
- Designated arbiter reviews and resolves the conflict
- Arbiter decides whether funds go to seller or are refunded to buyer
- Final resolution is recorded permanently on-chain

### 5. **Stellar Network Integration**

- Leverages the high speed and low cost of Stellar
- Built using the modern Soroban Smart Contract SDK
- Persistent storage with TTL management for long-running escrows
- Typed storage keys to prevent data conflicts across multiple escrows

---

## Contract Details

- **Network**: Stellar Testnet
- **Contract Address**: *(Deploy contract first using the steps below)*
- **Language**: Rust (Soroban SDK v21.0.0)
- **Storage Type**: Persistent (with TTL auto-extend)

---

## Contract Functions

| Function | Called By | Description |
|---|---|---|
| `top_up(user, amount)` | Anyone | Add simulation balance to an address before transacting |
| `get_balance(user)` | Anyone | Check the current simulation balance of an address |
| `create_escrow(buyer, seller, arbiter, amount, description)` | Buyer | Create a new escrow and lock buyer's simulation funds |
| `confirm_delivery(escrow_id, seller)` | Seller | Seller confirms readiness to ship, status → InProgress |
| `confirm_received(escrow_id, buyer)` | Buyer | Buyer confirms receipt of goods, funds released to seller |
| `raise_dispute(escrow_id, caller)` | Buyer / Seller | Open a dispute against the other party, status → Disputed |
| `resolve_dispute(escrow_id, arbiter, favor_seller)` | Arbiter | Arbiter settles the dispute — funds go to seller or refunded to buyer |
| `get_escrow(escrow_id)` | Anyone | Retrieve full details of an escrow by its ID |
| `get_all_ids()` | Anyone | Retrieve all escrow IDs ever created in this contract |
| `get_count()` | Anyone | Get the total number of escrows created |

---

## Escrow Status Flow

```
[create_escrow]
      │
      ▼
   PENDING
      │
      │ confirm_delivery()
      ▼
  IN_PROGRESS ──── raise_dispute() ───▶ DISPUTED
      │                                     │
      │ confirm_received()         resolve_dispute()
      ▼                               ┌────┴────┐
  COMPLETED                      COMPLETED   REFUNDED
  (funds to seller)            (favor=true) (favor=false)
```

---

## Getting Started

### Prerequisites

- Rust + `wasm32-unknown-unknown` target
- Stellar CLI (`stellar`)
- 3 wallet addresses on Stellar Testnet (buyer, seller, arbiter)

### Build & Deploy

```bash
# 1. Build the contract
cargo build --target wasm32-unknown-unknown --release

# 2. Deploy to Stellar Testnet
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/escrow_contract.wasm \
  --network testnet \
  --source <YOUR_SECRET_KEY>
```

### Step-by-Step Invoke

```bash
# Step 1 — Top up buyer's simulation balance
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <BUYER_SECRET_KEY> \
  -- top_up --user <BUYER_ADDRESS> --amount 10000000

# Step 2 — Create a new escrow
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <BUYER_SECRET_KEY> \
  -- create_escrow \
  --buyer <BUYER_ADDRESS> --seller <SELLER_ADDRESS> \
  --arbiter <ARBITER_ADDRESS> --amount 5000000 \
  --description "Buy second-hand laptop"

# Step 3 — Seller confirms shipment (save ESCROW_ID from step 2 output)
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <SELLER_SECRET_KEY> \
  -- confirm_delivery --escrow_id <ESCROW_ID> --seller <SELLER_ADDRESS>

# Step 4a — Buyer confirms receipt (normal flow)
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <BUYER_SECRET_KEY> \
  -- confirm_received --escrow_id <ESCROW_ID> --buyer <BUYER_ADDRESS>

# Step 4b — Open a dispute (conflict flow, alternative to step 4a)
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <BUYER_SECRET_KEY> \
  -- raise_dispute --escrow_id <ESCROW_ID> --caller <BUYER_ADDRESS>

# Step 5 — Arbiter resolves the dispute (false = refund to buyer)
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <ARBITER_SECRET_KEY> \
  -- resolve_dispute \
  --escrow_id <ESCROW_ID> --arbiter <ARBITER_ADDRESS> --favor_seller false

# Check balance at any time
stellar contract invoke \
  --id <CONTRACT_ID> --network testnet --source <ANY_SECRET_KEY> \
  -- get_balance --user <ADDRESS>
```

---

## Technical Requirements

- Soroban SDK v21.0.0
- Rust programming language
- Stellar Testnet / Mainnet

---

## Future Scope

### Short-Term Enhancements

1. **Real Token Integration**: Upgrade from simulation balance to real XLM transfers or custom Stellar Asset Contracts (SAC)
2. **Escrow Deadline**: Add a TTL-based timeout so escrows automatically expire if no action is taken within a set period
3. **Partial Release**: Support partial fund release (e.g. 50% after the first milestone is confirmed)
4. **Multi-Milestone Escrow**: Split a single deal into multiple staged payment steps

### Medium-Term Development

5. **Fee System**: Implement an arbiter fee that is automatically deducted from the escrow funds upon dispute resolution
6. **Escrow History per Address**: Index escrows by buyer or seller address to enable per-user transaction queries
7. **Reputation System**: Track each address's transaction record — number of completions versus disputes raised
8. **Notification Bridge**: Off-chain event listener to notify parties of any escrow status change

### Long-Term Vision

9. **Cross-Asset Escrow**: Support multiple token types and Stellar assets within a single escrow contract
10. **Decentralized Arbiter Pool**: DAO-based random and transparent arbiter selection for each dispute
11. **Zero-Knowledge Privacy**: Hide deal details from the public while keeping them cryptographically verifiable
12. **Cross-Chain Escrow**: Enable escrow across multiple blockchains using atomic swaps or bridge protocols
13. **AI Dispute Analyzer**: Optional AI integration to analyze dispute evidence before the arbiter makes a final decision
14. **Mobile DApp Integration**: Mobile frontend that connects directly to the Soroban smart contract

### Enterprise Features

15. **Corporate Procurement**: Adapt escrow for enterprise purchasing workflows with multi-approver authorization
16. **Immutable Audit Trail**: Log every status change permanently for financial compliance and auditing purposes
17. **Batch Escrow**: Create multiple escrow agreements in a single contract transaction
18. **SLA-Based Automation**: Auto-release funds when a Service Level Agreement condition is met, without requiring manual confirmation

---

## Running Tests

```bash
cargo test
```

Test coverage includes:
- Normal flow (create → delivery → received → completed)
- Dispute flow — buyer wins (refund)
- Dispute flow — seller wins (funds released)
- Validation: insufficient balance
- Validation: buyer and seller cannot be the same address


ID: CD47TRQHN2WZ2FWJ07E3ME566QGP7GH5LCQRGD5IG35XX3QV4WPVP55P
---

**Stellar Escrow DApp** - Securing Your Transactions on the Blockchain
