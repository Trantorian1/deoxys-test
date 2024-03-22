use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use bitvec::view::BitView;
use bonsai_trie::databases::{create_rocks_db, RocksDBConfig};
use bonsai_trie::id::{BasicId, BasicIdBuilder};
use bonsai_trie::BonsaiStorageConfig;
use bonsai_trie::{databases::RocksDB, BonsaiStorage};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use pathfinder_common::hash::PedersenHash;
use pathfinder_crypto::Felt as PathfinderFelt;
use pathfinder_merkle_tree::tree::{MerkleTree, TestStorage};
use pathfinder_storage::{Node, StoredNode};
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
    let block_range = 190..1000;

    // The contract to watch
    let contract_address = FieldElement::from_hex_be(
        "0x6a09ccb1caaecf3d9683efe335a667b2169a409d19c589ba1eb771cd210af75",
    )
    .unwrap();
    let mut id_builder = BasicIdBuilder::new();

    // ohhh... pretty 👀
    let bar = ProgressBar::new(block_range.end - block_range.start);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap(),
    );
    bar.println(format!("📜 checking for contract {contract_address:#x}"));
    let tempdir = tempdir().unwrap();
    let db = create_rocks_db(tempdir).unwrap();
    let mut persistent_bonsai_storage: BonsaiStorage<BasicId, _, Pedersen> = BonsaiStorage::new(
        RocksDB::<BasicId>::new(&db, RocksDBConfig::default()),
        BonsaiStorageConfig::default(),
    )
    .unwrap();
    let mut persistent_pathfinder_merkle_tree: MerkleTree<PedersenHash, 251> =
    pathfinder_merkle_tree::tree::MerkleTree::empty();
    let mut persistent_storage = pathfinder_merkle_tree::tree::TestStorage::default();

    for i in block_range {
        bar.inc(1);

        let state_update = get_state_update(&provider, i).await.unwrap();
        if let Some(storage_updates) = state_update.state_diff.storage_diffs.get(&contract_address)
        {
            bar.println(format!("🧱 block {i}"));
            save_storage_update(contract_address, storage_updates).await;

            // Special case to make the root for each key
            // if i == 470 || i == 469 {
            if false {
                persistent_and_pathfinder_storage(
                    contract_address,
                    &mut persistent_bonsai_storage,
                    &mut id_builder,
                    &mut persistent_pathfinder_merkle_tree,
                    &mut persistent_storage,
                    &bar,
                )
                .await;
            } else {
                //let bonsai_storage_root = bonsai_storage_root(contract_address, &bar).await;
                //let pathfinder_storage_root = pathfinder_storage_root(contract_address, &bar).await;
                let bonsai_storage_persistent_root = bonsai_storage_persistent_root(
                    contract_address,
                    &mut persistent_bonsai_storage,
                    &mut id_builder,
                    &bar,
                )
                .await;
                let pathfinder_storage_persistent_root = pathfinder_storage_persistent_root(contract_address, &mut persistent_pathfinder_merkle_tree, &mut persistent_storage, &bar).await;
                //bar.println(format!("🌳 storage root bonsai: {bonsai_storage_root:#064x}"));
                //bar.println(format!("🌳 storage root pathfinder: {pathfinder_storage_root:#064x}"));
                bar.println(format!(
                    "🌳 storage root persistent bonsai: {bonsai_storage_persistent_root:#064x}"
                ));
                bar.println(format!(
                    "🌳 storage root persistent pathfinder: {pathfinder_storage_persistent_root:#064x}"
                ));
                //assert_eq!(bonsai_storage_root, bonsai_storage_persistent_root);
                //assert_eq!(bonsai_storage_root, pathfinder_storage_root);
                assert_eq!(bonsai_storage_persistent_root, pathfinder_storage_persistent_root);
            }
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

async fn bonsai_storage_persistent_root(
    contract_address: FieldElement,
    persistent_bonsai_storage: &mut BonsaiStorage<BasicId, RocksDB<'_, BasicId>, Pedersen>,
    id_builder: &mut BasicIdBuilder,
    _bar: &ProgressBar,
) -> Felt {
    let contract_storage = CONTRACT_STORAGE.read().await;
    let contract_storage = contract_storage.get(&contract_address).unwrap();

    for (key, value) in contract_storage.read().await.iter() {
        //bar.println(format!("🔑 {key:#x} -> {value:#x}"));

        let key = key.to_bytes_be().view_bits()[5..].to_owned();
        let value = Felt::from_bytes_be(&value.to_bytes_be());

        persistent_bonsai_storage
            .insert(&contract_address.to_bytes_be(), &key, &value)
            .expect("Failed to insert into Bonsai storage")
    }

    persistent_bonsai_storage
        .commit(id_builder.new_id())
        .expect("Failed to commit to Bonsai storage");
    persistent_bonsai_storage
        .root_hash(&contract_address.to_bytes_be())
        .expect("Failed to retrieve root hash")
}

async fn bonsai_storage_root(contract_address: FieldElement, bar: &ProgressBar) -> Felt {
    let tempdir = tempdir().unwrap();
    let db = create_rocks_db(tempdir.path()).unwrap();
    let config = BonsaiStorageConfig::default();
    let mut bonsai_storage: BonsaiStorage<_, _, Pedersen> =
        BonsaiStorage::new(RocksDB::new(&db, RocksDBConfig::default()), config).unwrap();

    let contract_storage = CONTRACT_STORAGE.read().await;
    let contract_storage = contract_storage.get(&contract_address).unwrap();

    for (key, value) in contract_storage.read().await.iter() {
        //bar.println(format!("🔑 {key:#x} -> {value:#x}"));

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

async fn pathfinder_storage_persistent_root(
    contract_address: FieldElement,
    persistent_pathfinder_merkle_tree: &mut MerkleTree<PedersenHash, 251>,
    persistent_storage: &mut TestStorage,
    _bar: &ProgressBar,
) -> Felt {
    let contract_storage = CONTRACT_STORAGE.read().await;
    let contract_storage = contract_storage.get(&contract_address).unwrap();

    for (key, value) in contract_storage.read().await.iter() {
        //bar.println(format!("🔑 {key:#x} -> {value:#x}"));
        let key = key.to_bytes_be().view_bits()[5..].to_owned();
        let value = PathfinderFelt::from_be_slice(&value.to_bytes_be()).unwrap();

        persistent_pathfinder_merkle_tree.set(persistent_storage, key, value).unwrap();
    }

    let (felt, _) = commit_and_persist(persistent_pathfinder_merkle_tree.clone(), persistent_storage);
    Felt::from_hex(&felt.to_hex_str().into_owned()).unwrap()
}

async fn pathfinder_storage_root(contract_address: FieldElement, bar: &ProgressBar) -> Felt {
    let mut pathfinder_merkle_tree: MerkleTree<PedersenHash, 251> =
        pathfinder_merkle_tree::tree::MerkleTree::empty();
    let mut storage = pathfinder_merkle_tree::tree::TestStorage::default();
    let contract_storage = CONTRACT_STORAGE.read().await;
    let contract_storage = contract_storage.get(&contract_address).unwrap();

    for (key, value) in contract_storage.read().await.iter() {
        //bar.println(format!("🔑 {key:#x} -> {value:#x}"));
        let key = key.to_bytes_be().view_bits()[5..].to_owned();
        let value = PathfinderFelt::from_be_slice(&value.to_bytes_be()).unwrap();

        pathfinder_merkle_tree.set(&storage, key, value).unwrap();
    }

    let (felt, _) = commit_and_persist(pathfinder_merkle_tree.clone(), &mut storage);
    Felt::from_hex(&felt.to_hex_str().into_owned()).unwrap()
}

async fn persistent_and_pathfinder_storage(
    contract_address: FieldElement,
    persistent_bonsai_storage: &mut BonsaiStorage<BasicId, RocksDB<'_, BasicId>, Pedersen>,
    id_builder: &mut BasicIdBuilder,
    persistent_pathfinder_merkle_tree: &mut MerkleTree<PedersenHash, 251>,
    persistent_pathfinder_storage: &mut TestStorage,
    bar: &ProgressBar,
) {
    let contract_storage = CONTRACT_STORAGE.read().await;
    let contract_storage = contract_storage.get(&contract_address).unwrap();
    for (key, value) in contract_storage.read().await.iter() {
        bar.println(format!("🔑 {key:#x} -> {value:#x}"));
        let key = key.to_bytes_be().view_bits()[5..].to_owned();
        let value_pathfinder = PathfinderFelt::from_be_slice(&value.to_bytes_be()).unwrap();
        persistent_pathfinder_merkle_tree.set(persistent_pathfinder_storage, key.clone(), value_pathfinder).unwrap();
        let (felt, _) = commit_and_persist(persistent_pathfinder_merkle_tree.clone(), persistent_pathfinder_storage);
        let pathfinder_root_hash =Felt::from_hex(&felt.to_hex_str().into_owned()).unwrap();

        let value = Felt::from_bytes_be(&value.to_bytes_be());

        persistent_bonsai_storage
            .insert(&contract_address.to_bytes_be(), &key, &value)
            .expect("Failed to insert into Bonsai storage");
        persistent_bonsai_storage
            .commit(id_builder.new_id())
            .expect("Failed to commit to Bonsai storage");
        let bonsai_root_hash = persistent_bonsai_storage
            .root_hash(&contract_address.to_bytes_be())
            .expect("Failed to retrieve root hash");
        bar.println(format!(
            "🌳 storage root persistent: {bonsai_root_hash:#064x}"
        ));
        bar.println(format!(
            "🌳 storage root pathfinder: {pathfinder_root_hash:#064x}"
        ));
        assert_eq!(bonsai_root_hash, pathfinder_root_hash);
    }
}

/// Commits the tree changes and persists them to storage.
fn commit_and_persist(
    tree: MerkleTree<PedersenHash, 251>,
    storage: &mut TestStorage,
) -> (PathfinderFelt, u64) {
    use pathfinder_storage::Child;

    for (key, value) in &tree.leaves {
        let key = PathfinderFelt::from_bits(key).unwrap();
        storage.leaves.insert(key, *value);
    }

    let update = tree.commit(storage).unwrap();

    let mut indices = HashMap::new();
    let mut idx = storage.nodes.len();
    for hash in update.nodes.keys() {
        indices.insert(*hash, idx as u64);
        idx += 1;
    }

    for (hash, node) in update.nodes {
        let node = match node {
            Node::Binary { left, right } => {
                let left = match left {
                    Child::Id(idx) => idx,
                    Child::Hash(hash) => {
                        *indices.get(&hash).expect("Left child should have an index")
                    }
                };

                let right = match right {
                    Child::Id(idx) => idx,
                    Child::Hash(hash) => *indices
                        .get(&hash)
                        .expect("Right child should have an index"),
                };

                StoredNode::Binary { left, right }
            }
            Node::Edge { child, path } => {
                let child = match child {
                    Child::Id(idx) => idx,
                    Child::Hash(hash) => *indices.get(&hash).expect("Child should have an index"),
                };

                StoredNode::Edge { child, path }
            }
            Node::LeafBinary => StoredNode::LeafBinary,
            Node::LeafEdge { path } => StoredNode::LeafEdge { path },
        };

        storage
            .nodes
            .insert(*indices.get(&hash).unwrap(), (hash, node));
    }

    let index = *indices.get(&update.root).unwrap();

    (update.root, index)
}
