use bitvec::view::BitView;
use bonsai_trie::{
    databases::{create_rocks_db, RocksDB, RocksDBConfig},
    id::BasicIdBuilder,
    BonsaiStorage, BonsaiStorageConfig,
};
use starknet_types_core::{felt::Felt, hash::Pedersen};

pub fn main() {
    struct ContractState {
        address: &'static str,
        state_hash: &'static str,
    }

    let tempdir = tempfile::tempdir().unwrap();
    let db = create_rocks_db(tempdir.path()).unwrap();
    let config = BonsaiStorageConfig::default();
    let mut bonsai_storage: BonsaiStorage<_, _, Pedersen> =
        BonsaiStorage::new(RocksDB::new(&db, RocksDBConfig::default()), config).unwrap();

    let mut id_builder = BasicIdBuilder::new();

    // This is an exerpt of the storage for contract
    // 0x020cfa74ee3564b4cd5435cdace0f9c4d43b939620e4a0bb5076105df0a626c6 at block 4
    let contract_states = vec![
        ContractState {
            address: "0x0000000000000000000000000000000000000000000000000000000000000005",
            state_hash: "0x000000000000000000000000000000000000000000000000000000000000022b",
        },
        ContractState {
            address: "0x0313ad57fdf765addc71329abf8d74ac2bce6d46da8c2b9b82255a5076620300",
            state_hash: "0x04e7e989d58a17cd279eca440c5eaa829efb6f9967aaad89022acbe644c39b36",
        },
        // This seems to be what is causing the problem in case of double insertions.
        // Other value are fine
        ContractState {
            address: "0x313ad57fdf765addc71329abf8d74ac2bce6d46da8c2b9b82255a5076620301",
            state_hash: "0x453ae0c9610197b18b13645c44d3d0a407083d96562e8752aab3fab616cecb0",
        },
        ContractState {
            address: "0x05aee31408163292105d875070f98cb48275b8c87e80380b78d30647e05854d5",
            state_hash: "0x00000000000000000000000000000000000000000000000000000000000007e5",
        },
        ContractState {
            address: "0x06cf6c2f36d36b08e591e4489e92ca882bb67b9c39a3afccf011972a8de467f0",
            state_hash: "0x07ab344d88124307c07b56f6c59c12f4543e9c96398727854a322dea82c73240",
        },
    ];

    for contract_state in contract_states {
        let key = contract_state.address;
        let value = contract_state.state_hash;

        let key = Felt::from_hex(key).unwrap();
        let bitkey = key.to_bytes_be().view_bits()[5..].to_bitvec();
        let value = Felt::from_hex(value).unwrap();

        bonsai_storage
            .insert(&bitkey, &value)
            .expect("Failed to insert storage update into trie");

        // fails here for key 0x313ad57fdf765addc71329abf8d74ac2bce6d46da8c2b9b82255a5076620301
        // and value 0x453ae0c9610197b18b13645c44d3d0a407083d96562e8752aab3fab616cecb0
        bonsai_storage.insert(&bitkey, &value).expect(&format!(
            "Failed to insert storage update into trie for key {key:#x} & value {value:#x}"
        ));
    }

    let id = id_builder.new_id();
    bonsai_storage
        .commit(id)
        .expect("Failed to commit to bonsai storage");
    let root_hash = bonsai_storage.root_hash().expect("Failed to get root hash");

    println!("root hash: {root_hash:#x}");
}
