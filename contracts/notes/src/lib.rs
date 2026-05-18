#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype,
    Env, String, Address, Vec,
};

// ============================================================
// STATUS ESCROW
// ============================================================
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum EscrowStatus {
    Pending,      // Escrow dibuat, menunggu seller konfirmasi
    InProgress,   // Seller konfirmasi, barang sedang dikirim
    Completed,    // Buyer terima barang, dana cair ke seller
    Disputed,     // Ada sengketa, menunggu arbiter
    Refunded,     // Dana dikembalikan ke buyer
}

// ============================================================
// STRUKTUR DATA ESCROW
// ============================================================
#[contracttype]
#[derive(Clone, Debug)]
pub struct Escrow {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub arbiter: Address,
    pub amount: i128,         // Jumlah dana simulasi (tidak ada transfer asli)
    pub description: String,
    pub status: EscrowStatus,
}

// ============================================================
// STORAGE KEY — typed enum agar tidak conflict
// ============================================================
#[contracttype]
#[derive(Clone)]
pub enum StorageKey {
    Escrow(u64),        // data tiap escrow
    EscrowList,         // list semua ID escrow
    EscrowCount,        // total escrow dibuat
    Balance(Address),   // saldo simulasi tiap address
}

// TTL: ~1 jam minimum, ~24 jam maksimum di testnet
const TTL_LOW: u32  = 1_000;
const TTL_HIGH: u32 = 17_280;

// ============================================================
// HELPER: baca & tulis saldo simulasi
// ============================================================
fn get_balance(env: &Env, addr: &Address) -> i128 {
    let key = StorageKey::Balance(addr.clone());
    env.storage().persistent().get(&key).unwrap_or(0i128)
}

fn set_balance(env: &Env, addr: &Address, amount: i128) {
    let key = StorageKey::Balance(addr.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);
}

// ============================================================
// CONTRACT
// ============================================================
#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {

    // ----------------------------------------------------------
    // TOP UP — isi saldo simulasi sebelum bertransaksi
    // Di testnet nyata tidak ada fungsi ini; ini hanya untuk sim
    // ----------------------------------------------------------
    pub fn top_up(env: Env, user: Address, amount: i128) -> i128 {
        user.require_auth();

        if amount <= 0 {
            panic!("Amount top up harus lebih dari 0");
        }

        let current = get_balance(&env, &user);
        let new_balance = current + amount;
        set_balance(&env, &user, new_balance);

        return new_balance;
    }

    // ----------------------------------------------------------
    // CEK SALDO SIMULASI
    // ----------------------------------------------------------
    pub fn get_balance(env: Env, user: Address) -> i128 {
        return get_balance(&env, &user);
    }

    // ----------------------------------------------------------
    // CREATE ESCROW
    // Saldo buyer dikurangi, dicatat di storage contract
    // ----------------------------------------------------------
    pub fn create_escrow(
        env: Env,
        buyer: Address,
        seller: Address,
        arbiter: Address,
        amount: i128,
        description: String,
    ) -> u64 {
        buyer.require_auth();

        // Validasi input
        if amount <= 0 {
            panic!("Amount harus lebih dari 0");
        }
        if buyer == seller {
            panic!("Buyer dan seller tidak boleh sama");
        }
        if buyer == arbiter || seller == arbiter {
            panic!("Arbiter tidak boleh sama dengan buyer atau seller");
        }

        // Cek saldo buyer cukup
        let buyer_balance = get_balance(&env, &buyer);
        if buyer_balance < amount {
            panic!("Saldo buyer tidak cukup. Top up dulu!");
        }

        // Kurangi saldo buyer (simulasi lock dana)
        set_balance(&env, &buyer, buyer_balance - amount);

        // Generate ID unik
        let id: u64 = env.prng().gen::<u64>();

        // Buat objek escrow
        let escrow = Escrow {
            id,
            buyer: buyer.clone(),
            seller,
            arbiter,
            amount,
            description,
            status: EscrowStatus::Pending,
        };

        // Simpan escrow
        let key = StorageKey::Escrow(id);
        env.storage().persistent().set(&key, &escrow);
        env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);

        // Tambah ke list
        let list_key = StorageKey::EscrowList;
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&list_key)
            .unwrap_or(Vec::new(&env));
        ids.push_back(id);
        env.storage().persistent().set(&list_key, &ids);
        env.storage().persistent().extend_ttl(&list_key, TTL_LOW, TTL_HIGH);

        // Update counter
        let count_key = StorageKey::EscrowCount;
        let count: u64 = env
            .storage()
            .persistent()
            .get(&count_key)
            .unwrap_or(0u64);
        env.storage().persistent().set(&count_key, &(count + 1));
        env.storage().persistent().extend_ttl(&count_key, TTL_LOW, TTL_HIGH);

        return id;
    }

    // ----------------------------------------------------------
    // CONFIRM DELIVERY (oleh Seller)
    // Pending -> InProgress
    // ----------------------------------------------------------
    pub fn confirm_delivery(env: Env, escrow_id: u64, seller: Address) -> String {
        seller.require_auth();

        let key = StorageKey::Escrow(escrow_id);
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Escrow tidak ditemukan");

        if escrow.seller != seller {
            panic!("Hanya seller yang bisa konfirmasi pengiriman");
        }
        if escrow.status != EscrowStatus::Pending {
            panic!("Status harus Pending");
        }

        escrow.status = EscrowStatus::InProgress;
        env.storage().persistent().set(&key, &escrow);
        env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);

        return String::from_str(&env, "Pengiriman dikonfirmasi. Status: InProgress");
    }

    // ----------------------------------------------------------
    // CONFIRM RECEIVED (oleh Buyer)
    // InProgress -> Completed, saldo cair ke seller
    // ----------------------------------------------------------
    pub fn confirm_received(env: Env, escrow_id: u64, buyer: Address) -> String {
        buyer.require_auth();

        let key = StorageKey::Escrow(escrow_id);
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Escrow tidak ditemukan");

        if escrow.buyer != buyer {
            panic!("Hanya buyer yang bisa konfirmasi penerimaan");
        }
        if escrow.status != EscrowStatus::InProgress {
            panic!("Status harus InProgress");
        }

        // Cairkan dana simulasi ke seller
        let seller_balance = get_balance(&env, &escrow.seller);
        set_balance(&env, &escrow.seller, seller_balance + escrow.amount);

        escrow.status = EscrowStatus::Completed;
        env.storage().persistent().set(&key, &escrow);
        env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);

        return String::from_str(&env, "Barang diterima! Dana simulasi berhasil dikirim ke seller.");
    }

    // ----------------------------------------------------------
    // RAISE DISPUTE (oleh Buyer atau Seller)
    // InProgress -> Disputed
    // ----------------------------------------------------------
    pub fn raise_dispute(env: Env, escrow_id: u64, caller: Address) -> String {
        caller.require_auth();

        let key = StorageKey::Escrow(escrow_id);
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Escrow tidak ditemukan");

        if escrow.buyer != caller && escrow.seller != caller {
            panic!("Hanya buyer atau seller yang bisa buka sengketa");
        }
        if escrow.status != EscrowStatus::InProgress {
            panic!("Sengketa hanya bisa dibuka saat status InProgress");
        }

        escrow.status = EscrowStatus::Disputed;
        env.storage().persistent().set(&key, &escrow);
        env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);

        return String::from_str(&env, "Sengketa dibuka. Menunggu keputusan arbiter.");
    }

    // ----------------------------------------------------------
    // RESOLVE DISPUTE (oleh Arbiter)
    // favor_seller=true  -> dana ke seller (Completed)
    // favor_seller=false -> refund ke buyer (Refunded)
    // ----------------------------------------------------------
    pub fn resolve_dispute(
        env: Env,
        escrow_id: u64,
        arbiter: Address,
        favor_seller: bool,
    ) -> String {
        arbiter.require_auth();

        let key = StorageKey::Escrow(escrow_id);
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Escrow tidak ditemukan");

        if escrow.arbiter != arbiter {
            panic!("Hanya arbiter yang bisa menyelesaikan sengketa");
        }
        if escrow.status != EscrowStatus::Disputed {
            panic!("Status harus Disputed");
        }

        if favor_seller {
            let seller_balance = get_balance(&env, &escrow.seller);
            set_balance(&env, &escrow.seller, seller_balance + escrow.amount);
            escrow.status = EscrowStatus::Completed;
            env.storage().persistent().set(&key, &escrow);
            env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);
            return String::from_str(&env, "Sengketa selesai: Dana simulasi dikirim ke seller.");
        } else {
            let buyer_balance = get_balance(&env, &escrow.buyer);
            set_balance(&env, &escrow.buyer, buyer_balance + escrow.amount);
            escrow.status = EscrowStatus::Refunded;
            env.storage().persistent().set(&key, &escrow);
            env.storage().persistent().extend_ttl(&key, TTL_LOW, TTL_HIGH);
            return String::from_str(&env, "Sengketa selesai: Dana simulasi dikembalikan ke buyer.");
        }
    }

    // ----------------------------------------------------------
    // GET ESCROW BY ID
    // ----------------------------------------------------------
    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        let key = StorageKey::Escrow(escrow_id);
        env.storage()
            .persistent()
            .get(&key)
            .expect("Escrow tidak ditemukan")
    }

    // ----------------------------------------------------------
    // GET ALL IDs
    // ----------------------------------------------------------
    pub fn get_all_ids(env: Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&StorageKey::EscrowList)
            .unwrap_or(Vec::new(&env))
    }

    // ----------------------------------------------------------
    // GET TOTAL COUNT
    // ----------------------------------------------------------
    pub fn get_count(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::EscrowCount)
            .unwrap_or(0u64)
    }
}

// ============================================================
// UNIT TEST
// ============================================================
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address, Address) {
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(env, &contract_id);
        let buyer   = Address::generate(env);
        let seller  = Address::generate(env);
        let arbiter = Address::generate(env);
        (client, buyer, seller, arbiter, contract_id)
    }

    #[test]
    fn test_top_up_dan_cek_saldo() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, buyer, _, _, _) = setup(&env);

        assert_eq!(client.get_balance(&buyer), 0);
        let saldo = client.top_up(&buyer, &10_000_000i128);
        assert_eq!(saldo, 10_000_000);
        assert_eq!(client.get_balance(&buyer), 10_000_000);
    }

    #[test]
    fn test_alur_normal() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, buyer, seller, arbiter, _) = setup(&env);

        client.top_up(&buyer, &10_000_000i128);

        let id = client.create_escrow(
            &buyer, &seller, &arbiter,
            &5_000_000i128,
            &String::from_str(&env, "Beli laptop bekas"),
        );

        // Saldo buyer berkurang setelah create
        assert_eq!(client.get_balance(&buyer), 5_000_000);

        client.confirm_delivery(&id, &seller);
        client.confirm_received(&id, &buyer);

        let escrow = client.get_escrow(&id);
        assert!(escrow.status == EscrowStatus::Completed);

        // Dana sudah cair ke seller
        assert_eq!(client.get_balance(&seller), 5_000_000);
    }

    #[test]
    fn test_sengketa_buyer_menang() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, buyer, seller, arbiter, _) = setup(&env);

        client.top_up(&buyer, &10_000_000i128);

        let id = client.create_escrow(
            &buyer, &seller, &arbiter,
            &5_000_000i128,
            &String::from_str(&env, "Beli HP second"),
        );

        client.confirm_delivery(&id, &seller);
        client.raise_dispute(&id, &buyer);
        client.resolve_dispute(&id, &arbiter, &false);

        let escrow = client.get_escrow(&id);
        assert!(escrow.status == EscrowStatus::Refunded);

        // Dana kembali ke buyer
        assert_eq!(client.get_balance(&buyer), 10_000_000);
        assert_eq!(client.get_balance(&seller), 0);
    }

    #[test]
    fn test_sengketa_seller_menang() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, buyer, seller, arbiter, _) = setup(&env);

        client.top_up(&buyer, &10_000_000i128);

        let id = client.create_escrow(
            &buyer, &seller, &arbiter,
            &5_000_000i128,
            &String::from_str(&env, "Beli kamera"),
        );

        client.confirm_delivery(&id, &seller);
        client.raise_dispute(&id, &seller);
        client.resolve_dispute(&id, &arbiter, &true);

        let escrow = client.get_escrow(&id);
        assert!(escrow.status == EscrowStatus::Completed);
        assert_eq!(client.get_balance(&seller), 5_000_000);
    }

    #[test]
    #[should_panic(expected = "Saldo buyer tidak cukup")]
    fn test_saldo_tidak_cukup() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, buyer, seller, arbiter, _) = setup(&env);

        // Tidak top up dulu, langsung create escrow
        client.create_escrow(
            &buyer, &seller, &arbiter,
            &5_000_000i128,
            &String::from_str(&env, "Harusnya gagal"),
        );
    }

    #[test]
    #[should_panic(expected = "Buyer dan seller tidak boleh sama")]
    fn test_buyer_seller_sama() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, buyer, _, arbiter, _) = setup(&env);

        client.top_up(&buyer, &10_000_000i128);
        client.create_escrow(
            &buyer, &buyer, &arbiter,
            &1_000_000i128,
            &String::from_str(&env, "Invalid"),
        );
    }
}