#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Symbol,
};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum InvoiceStatus {
    Pending,
    Funded,
    Paid,
    Defaulted,
}

#[contracttype]
#[derive(Clone)]
pub struct Invoice {
    pub id: u64,
    pub owner: Address,
    pub debtor: String,
    pub amount: i128,
    pub due_date: u64,
    pub description: String,
    pub status: InvoiceStatus,
    pub created_at: u64,
    pub funded_at: u64,
    pub paid_at: u64,
    pub pool_contract: Address,
}

/// Wallet / explorer–oriented view derived from [`Invoice`] (no extra storage).
/// Field names align with common JSON token metadata (`name`, `description`, `image`)
/// plus invoice-specific attributes; see `contracts/invoice/README.md` for SEP notes.
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub struct InvoiceMetadata {
    pub name: String,
    pub description: String,
    /// Placeholder asset URI until per-invoice art exists.
    pub image: String,
    pub amount: i128,
    pub debtor: String,
    pub due_date: u64,
    pub status: InvoiceStatus,
    /// Short ticker, SEP-0041–style (e.g. `INV-1`).
    pub symbol: String,
    /// Smallest units per whole token for `amount` (USDC on Stellar uses 7).
    pub decimals: u32,
}

#[contracttype]
pub enum DataKey {
    Invoice(u64),
    InvoiceCount,
    Admin,
    Pool,
    Initialized,
}

const EVT: Symbol = symbol_short!("INVOICE");

/// Writes decimal digits of `n` into `buf` (left-aligned), returns digit count.
fn write_u64_decimal(buf: &mut [u8], mut n: u64) -> usize {
    if n == 0 {
        if buf.is_empty() {
            return 0;
        }
        buf[0] = b'0';
        return 1;
    }
    let mut i = 0usize;
    while n > 0 {
        if i >= buf.len() {
            break;
        }
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    let mut lo = 0usize;
    let mut hi = i - 1;
    while lo < hi {
        buf.swap(lo, hi);
        lo += 1;
        hi -= 1;
    }
    i
}

fn concat_prefix_u64(env: &Env, prefix: &[u8], id: u64) -> String {
    let mut buf = [0u8; 40];
    let plen = prefix.len();
    buf[..plen].copy_from_slice(prefix);
    let dlen = write_u64_decimal(&mut buf[plen..], id);
    String::from_bytes(env, &buf[..plen + dlen])
}

#[contract]
pub struct InvoiceContract;

#[contractimpl]
impl InvoiceContract {
    pub fn initialize(env: Env, admin: Address, pool: Address) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Pool, &pool);
        env.storage().instance().set(&DataKey::InvoiceCount, &0u64);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    /// SME creates a new invoice token on-chain
    pub fn create_invoice(
        env: Env,
        owner: Address,
        debtor: String,
        amount: i128,
        due_date: u64,
        description: String,
    ) -> u64 {
        owner.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }
        if due_date <= env.ledger().timestamp() {
            panic!("due date must be in the future");
        }

        let count: u64 = env.storage().instance().get(&DataKey::InvoiceCount).unwrap_or(0);
        let id = count + 1;

        // Placeholder address — real pool address set on fund_invoice
        let placeholder: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        let invoice = Invoice {
            id,
            owner: owner.clone(),
            debtor,
            amount,
            due_date,
            description,
            status: InvoiceStatus::Pending,
            created_at: env.ledger().timestamp(),
            funded_at: 0,
            paid_at: 0,
            pool_contract: placeholder,
        };

        env.storage().persistent().set(&DataKey::Invoice(id), &invoice);
        env.storage().instance().set(&DataKey::InvoiceCount, &id);
        env.events().publish((EVT, symbol_short!("created")), (id, owner, amount));

        id
    }

    /// Called by the pool contract when it funds an invoice
    pub fn mark_funded(env: Env, id: u64, pool: Address) {
        pool.require_auth();

        let authorized_pool: Address = env.storage().instance().get(&DataKey::Pool).expect("not initialized");
        if pool != authorized_pool {
            panic!("unauthorized pool");
        }

        let mut invoice: Invoice = env.storage()
            .persistent()
            .get(&DataKey::Invoice(id))
            .expect("invoice not found");

        if invoice.status != InvoiceStatus::Pending {
            panic!("invoice is not pending");
        }

        invoice.status = InvoiceStatus::Funded;
        invoice.funded_at = env.ledger().timestamp();
        invoice.pool_contract = pool;

        env.storage().persistent().set(&DataKey::Invoice(id), &invoice);
        env.events().publish((EVT, symbol_short!("funded")), id);
    }

    /// Called by the pool when repayment is confirmed
    pub fn mark_paid(env: Env, id: u64, caller: Address) {
        caller.require_auth();

        let pool: Address = env.storage().instance().get(&DataKey::Pool).expect("not initialized");
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");

        let mut invoice: Invoice = env.storage()
            .persistent()
            .get(&DataKey::Invoice(id))
            .expect("invoice not found");

        if caller != invoice.owner && caller != pool && caller != admin {
            panic!("unauthorized");
        }
        if invoice.status != InvoiceStatus::Funded {
            panic!("invoice is not funded");
        }

        invoice.status = InvoiceStatus::Paid;
        invoice.paid_at = env.ledger().timestamp();

        env.storage().persistent().set(&DataKey::Invoice(id), &invoice);
        env.events().publish((EVT, symbol_short!("paid")), id);
    }

    /// Mark invoice as defaulted (missed due date, no repayment)
    pub fn mark_defaulted(env: Env, id: u64, pool: Address) {
        pool.require_auth();

        let authorized_pool: Address = env.storage().instance().get(&DataKey::Pool).expect("not initialized");
        if pool != authorized_pool {
            panic!("unauthorized pool");
        }

        let mut invoice: Invoice = env.storage()
            .persistent()
            .get(&DataKey::Invoice(id))
            .expect("invoice not found");

        if invoice.status != InvoiceStatus::Funded {
            panic!("invoice is not funded");
        }

        invoice.status = InvoiceStatus::Defaulted;
        env.storage().persistent().set(&DataKey::Invoice(id), &invoice);
        env.events().publish((EVT, symbol_short!("default")), id);
    }

    pub fn get_invoice(env: Env, id: u64) -> Invoice {
        env.storage()
            .persistent()
            .get(&DataKey::Invoice(id))
            .expect("invoice not found")
    }

    /// SEP-oriented metadata for invoice id `id` (same ledger fields as `get_invoice`).
    pub fn get_metadata(env: Env, id: u64) -> InvoiceMetadata {
        let inv: Invoice = env
            .storage()
            .persistent()
            .get(&DataKey::Invoice(id))
            .expect("invoice not found");

        let name = concat_prefix_u64(&env, b"Astera Invoice #", inv.id);
        let symbol = concat_prefix_u64(&env, b"INV-", inv.id);
        let image = String::from_str(&env, "https://astera.io/metadata/invoice/placeholder.svg");

        InvoiceMetadata {
            name,
            description: inv.description.clone(),
            image,
            amount: inv.amount,
            debtor: inv.debtor.clone(),
            due_date: inv.due_date,
            status: inv.status.clone(),
            symbol,
            decimals: 7,
        }
    }

    pub fn get_invoice_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::InvoiceCount).unwrap_or(0)
    }

    /// Update authorized pool address (admin only)
    pub fn set_pool(env: Env, admin: Address, pool: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("not initialized");
        if admin != stored_admin {
            panic!("unauthorized");
        }
        env.storage().instance().set(&DataKey::Pool, &pool);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_create_and_fund_invoice() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, InvoiceContract);
        let client = InvoiceContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let pool = Address::generate(&env);
        let sme = Address::generate(&env);

        client.initialize(&admin, &pool);

        let id = client.create_invoice(
            &sme,
            &String::from_str(&env, "ACME Corp"),
            &1_000_000_000i128, // 1000 USDC (7 decimals)
            &(env.ledger().timestamp() + 2_592_000), // 30 days
            &String::from_str(&env, "Invoice #001 - Goods delivery"),
        );

        assert_eq!(id, 1);

        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Pending);

        let meta = client.get_metadata(&id);
        assert_eq!(meta.status, InvoiceStatus::Pending);
        assert_eq!(meta.amount, 1_000_000_000i128);
        assert_eq!(meta.decimals, 7u32);
        assert_eq!(meta.symbol, String::from_str(&env, "INV-1"));
        assert_eq!(meta.name, String::from_str(&env, "Astera Invoice #1"));

        client.mark_funded(&id, &pool);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Funded);
        assert_eq!(client.get_metadata(&id).status, InvoiceStatus::Funded);

        client.mark_paid(&id, &sme);
        let invoice = client.get_invoice(&id);
        assert_eq!(invoice.status, InvoiceStatus::Paid);
        assert_eq!(client.get_metadata(&id).status, InvoiceStatus::Paid);
    }
}
