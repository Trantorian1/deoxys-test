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

    let db1 = create_rocks_db("./rocksdb1").unwrap();
    let config1 = BonsaiStorageConfig::default();
    let mut bonsai_storage1: BonsaiStorage<_, _, Pedersen> =
        BonsaiStorage::new(RocksDB::new(&db1, RocksDBConfig::default()), config1).unwrap();

    let db2 = create_rocks_db("./rocksdb2").unwrap();
    let config2 = BonsaiStorageConfig::default();
    let mut bonsai_storage2: BonsaiStorage<_, _, Pedersen> =
        BonsaiStorage::new(RocksDB::new(&db2, RocksDBConfig::default()), config2).unwrap();

    let mut id_builder = BasicIdBuilder::new();

    let contract_states = vec![
        ContractState {
            address: "0x020cfa74ee3564b4cd5435cdace0f9c4d43b939620e4a0bb5076105df0a626c6",
            state_hash: "0x3a1606fc1a168e11bc31605aa32265a1a887c185feebb255a56bcac189fd5b6",
        },
        ContractState {
            address: "0x031c887d82502ceb218c06ebb46198da3f7b92864a8223746bc836dda3e34b52",
            state_hash: "0x1f881354625568925870a49ce72c4e51dc0a5b7799d6d072f457b886ee49743",
        },
        ContractState {
            address: "0x031c9cdb9b00cb35cf31c05855c0ec3ecf6f7952a1ce6e3c53c3455fcd75a280",
            state_hash: "0x77acb87553348ab4da75f6264446dce1820d6a1577c7685d5ca70d34b836373",
        },
        ContractState {
            address: "0x06ee3440b08a9c805305449ec7f7003f27e9f7e287b83610952ec36bdc5a6bae",
            state_hash: "0x4fc78cbac87f833e56c91dfd6eda5be3362204d86d24f1e1e81577d509f963b",
        },
        ContractState {
            address: "0x0735596016a37ee972c42adef6a3cf628c19bb3794369c65d2c82ba034aecf2c",
            state_hash: "0x6c82bcd10124bf2c6a832c1329edffc750571a0e97a859af5b0aef12936eb13",
        },
    ];

    for contract_state in contract_states {
        let key = contract_state.address;
        let value = contract_state.state_hash;
        let key = Felt::from_hex(key).unwrap().to_bytes_be().view_bits()[5..].to_bitvec();
        let value = Felt::from_hex(value).unwrap();
        bonsai_storage1
            .insert(&key, &value)
            .expect("Failed to insert storage update into trie");
        bonsai_storage2
            .insert(&key, &value)
            .expect("Failed to insert storage update into trie");
    }

    let id = id_builder.new_id();
    bonsai_storage1
        .commit(id)
        .expect("Failed to commit to bonsai storage");
    let root_hash = bonsai_storage1
        .root_hash()
        .expect("Failed to get root hash");

    println!("root hash 0: {root_hash:#x}");

    let contract_states = vec![
        ContractState {
            address: "0x06538fdd3aa353af8a87f5fe77d1f533ea82815076e30a86d65b72d3eb4f0b80",
            state_hash: "0x2acf9d2ae5a475818075672b04e317e9da3d5180fed2c5f8d6d8a5fd5a92257",
        },
        ContractState {
            address: "0x0327d34747122d7a40f4670265b098757270a449ec80c4871450fffdab7c2fa8",
            state_hash: "0x100bd6fbfced88ded1b34bd1a55b747ce3a9fde9a914bca75571e4496b56443",
        },
    ];

    for contract_state in contract_states {
        let key = contract_state.address;
        let value = contract_state.state_hash;
        let key = Felt::from_hex(key).unwrap().to_bytes_be().view_bits()[5..].to_bitvec();
        let value = Felt::from_hex(value).unwrap();

        bonsai_storage1
            .insert(&key, &value)
            .expect("Failed to insert storage update into trie");
        bonsai_storage2
            .insert(&key, &value)
            .expect("Failed to insert storage update into trie");
    }

    let id = id_builder.new_id();
    bonsai_storage1
        .commit(id)
        .expect("Failed to commit to bonsai storage");
    let root_hash = bonsai_storage1
        .root_hash()
        .expect("Failed to get root hash");

    println!("root hash 1: {root_hash:#x}");

    let contract_states = vec![
        ContractState {
            address: "0x001fb4457f3fe8a976bdb9c04dd21549beeeb87d3867b10effe0c4bd4064a8e4",
            state_hash: "0x00a038cda302fedbc4f6117648c6d3faca3cda924cb9c517b46232c6316b152f",
        },
        ContractState {
            address: "0x05790719f16afe1450b67a92461db7d0e36298d6a5f8bab4f7fd282050e02f4f",
            state_hash: "0x02808c7d8f3745e55655ad3f51f096d0c06a41f3d76caf96bad80f9be9ced171",
        },
        ContractState {
            address: "0x057b973bf2eb26ebb28af5d6184b4a044b24a8dcbf724feb95782c4d1aef1ca9",
            state_hash: "0x011a08db805b8322d953f07903d419703badb7a4c97c6dc474caa3cd21b5b44b",
        },
        ContractState {
            address: "0x02d6c9569dea5f18628f1ef7c15978ee3093d2d3eec3b893aac08004e678ead3",
            state_hash: "0x07036d8dd68dc9539c6db8c88f72b1ab16e76d62b5f09118eca5ae78276b0ee4",
        },
    ];

    for contract_state in contract_states {
        let key = contract_state.address;
        let value = contract_state.state_hash;
        let key = Felt::from_hex(key).unwrap().to_bytes_be().view_bits()[5..].to_bitvec();
        let value = Felt::from_hex(value).unwrap();
        bonsai_storage1
            .insert(&key, &value)
            .expect("Failed to insert storage update into trie");
        bonsai_storage2
            .insert(&key, &value)
            .expect("Failed to insert storage update into trie");
    }

    let id = id_builder.new_id();
    bonsai_storage1
        .commit(id)
        .expect("Failed to commit to bonsai storage");
    let root_hash = bonsai_storage1
        .root_hash()
        .expect("Failed to get root hash");

    println!("root hash 2: {root_hash:#x}");

    bonsai_storage2
        .commit(id)
        .expect("Failed to commit to bonsai storage");
    let root_hash = bonsai_storage2
        .root_hash()
        .expect("Failed to get root hash");

    println!("root hash ': {root_hash:#x}");
}
