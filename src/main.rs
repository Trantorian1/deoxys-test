use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use bitvec::view::BitView;
use bonsai_trie::databases::{create_rocks_db, RocksDBConfig};
use bonsai_trie::id::BasicIdBuilder;
use bonsai_trie::BonsaiStorageConfig;
use bonsai_trie::{databases::RocksDB, BonsaiStorage};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use starknet::core::types::FieldElement;
use starknet::providers::{
    sequencer::models::{
        state_update::StorageDiff,
        BlockId::{self},
        StateUpdate,
    },
    SequencerGatewayProvider,
};
use starknet_types_core::felt::Felt;
use starknet_types_core::hash::Pedersen;
use tempfile::tempdir;
use tokio::sync::RwLock;

lazy_static! {
    static ref CONTRACT_STORAGE: RwLock<HashMap<FieldElement, RwLock<HashMap<FieldElement, FieldElement>>>> =
        RwLock::new(HashMap::new());
}

const IDENTIFIER: &[u8; 10] = b"0xcontract";

#[tokio::main]
async fn main() {
    let provider = SequencerGatewayProvider::starknet_alpha_mainnet();

    // Change this to update the range of blocks to test
    // NOTE: This should contain the block at which `contract_address` was defined
    let block_range = 0..400;

    // The contract to watch
    let contract_address = FieldElement::from_hex_be(
        "0x020cfa74ee3564b4cd5435cdace0f9c4d43b939620e4a0bb5076105df0a626c6",
    )
    .unwrap();

    // ohhh... pretty 👀
    let bar = ProgressBar::new(block_range.end - block_range.start);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap(),
    );
    bar.println(format!("📜 checking for contract {contract_address:#x}"));

    for i in block_range {
        bar.inc(1);

        let state_update = get_state_update(&provider, i).await.unwrap();
        if let Some(storage_updates) = state_update.state_diff.storage_diffs.get(&contract_address)
        {
            bar.println(format!("🧱 block {i}"));
            save_storage_update(contract_address, storage_updates).await;

            let storage_root = storage_root(contract_address, &bar).await;
            bar.println(format!("🌳 storage root: {storage_root:#064x}"));
        }
    }

    bar.finish();
}

async fn get_state_update(
    provider: &SequencerGatewayProvider,
    i: u64,
) -> anyhow::Result<StateUpdate> {
    let mut retries = 15;

    while retries > 0 {
        if let Ok(state_update) = provider.get_state_update(BlockId::Number(i)).await {
            return Ok(state_update);
        }

        retries -= 1;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Err(anyhow!("Failed to retrieve state update for block {i}"))
}

async fn save_storage_update(contract_address: FieldElement, storage_updates: &[StorageDiff]) {
    let storage_new: HashMap<FieldElement, FieldElement> = storage_updates
        .iter()
        .map(|StorageDiff { key, value }| (*key, *value))
        .collect();

    let mut contract_storage = CONTRACT_STORAGE.write().await;

    match contract_storage.get(&contract_address) {
        Some(storage_old) => {
            storage_old.write().await.extend(storage_new);
        }
        None => {
            contract_storage.insert(contract_address, RwLock::new(storage_new));
        }
    };
}

async fn storage_root(contract_address: FieldElement, bar: &ProgressBar) -> Felt {
    let tempdir = tempdir().unwrap();
    let db = create_rocks_db(tempdir.path()).unwrap();
    let config = BonsaiStorageConfig::default();
    let mut bonsai_storage: BonsaiStorage<_, _, Pedersen> =
        BonsaiStorage::new(RocksDB::new(&db, RocksDBConfig::default()), config).unwrap();

    let contract_storage = CONTRACT_STORAGE.read().await;
    let contract_storage = contract_storage.get(&contract_address).unwrap();

    for (key, value) in contract_storage.read().await.iter() {
        bar.println(format!("🔑 {key:#x} -> {value:#x}"));

        let key = key.to_bytes_be().view_bits()[5..].to_owned();
        let value = Felt::from_bytes_be(&value.to_bytes_be());

        bonsai_storage
            .insert(IDENTIFIER, &key, &value)
            .expect("Failed to insert into Bonsai storage")
    }

    let mut id_builder = BasicIdBuilder::new();
    bonsai_storage
        .commit(id_builder.new_id())
        .expect("Failed to commit to Bonsai storage");
    bonsai_storage
        .root_hash(IDENTIFIER)
        .expect("Failed to retrieve root hash")
}
