use crate::batch_handler::block_type_helpers::{
    decode_initial_patch, decode_patch, encode_initial_patch,
};
use crate::batch_handler::block_types::Block;
use crate::database;
use crate::encryption::batch::{BatchData, DecryptedBatch};
use dashmap::DashMap;
use futures::future::join_all;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

const SNAPSHOT_INTERVAL: usize = 100;
const CACHE_SIZE_ENTRIES: u64 = 500_000; // ~500MB - ~1GB

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockData {
    pub timestamp: u128,
    pub block_id: u64,
    pub block: Block,
    pub seq: u64, // Global sequence number (patch count) for this block state
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocumentData {
    pub doc_id: String,
    pub blocks: Vec<BlockData>,
}

type CacheKey = (String, u64, u128); // (doc_id, block_id, timestamp)
type IndexKey = (String, u64); // (doc_id, block_id)

/// Maps (doc_id, block_id) -> BTreeMap<timestamp, seq>
/// Allows us to find consecutive cached states without DB queries
static CACHE_INDEX: LazyLock<DashMap<IndexKey, BTreeMap<u128, u64>>> = LazyLock::new(DashMap::new);

/// Tracks the latest known state per (doc_id, block_id): (timestamp, seq)
/// When cache has this entry, we can skip DB queries for "get latest" calls
static KNOWN_LATEST: LazyLock<DashMap<IndexKey, (u128, u64)>> = LazyLock::new(DashMap::new);

pub static BLOCK_CACHE: LazyLock<Cache<CacheKey, BlockData>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(CACHE_SIZE_ENTRIES)
        .time_to_idle(std::time::Duration::from_secs(300))
        .eviction_listener(|key: Arc<CacheKey>, _value, _cause| {
            let index_key = (key.0.clone(), key.1);
            if let Some(mut index) = CACHE_INDEX.get_mut(&index_key) {
                index.remove(&key.2);
                if index.is_empty() {
                    drop(index);
                    CACHE_INDEX.remove(&index_key);
                }
            }
            // If evicted timestamp was our known latest, clear it
            if let Some(known) = KNOWN_LATEST.get(&index_key) {
                if known.0 == key.2 {
                    drop(known);
                    KNOWN_LATEST.remove(&index_key);
                }
            }
        })
        .build()
});

/// Find best cached timestamp <= target_ts, returns (timestamp, seq)
fn find_best_cached(doc_id: &str, block_id: u64, target_ts: u128) -> Option<(u128, u64)> {
    let index_key = (doc_id.to_string(), block_id);
    CACHE_INDEX.get(&index_key).and_then(|index| {
        index
            .range(..=target_ts)
            .next_back()
            .map(|(&ts, &seq)| (ts, seq))
    })
}

/// Find the cached entry with smallest timestamp > target_ts, returns (timestamp, seq)
fn find_next_cached(doc_id: &str, block_id: u64, target_ts: u128) -> Option<(u128, u64)> {
    let index_key = (doc_id.to_string(), block_id);
    CACHE_INDEX.get(&index_key).and_then(|index| {
        index
            .range((
                std::ops::Bound::Excluded(target_ts),
                std::ops::Bound::Unbounded,
            ))
            .next()
            .map(|(&ts, &seq)| (ts, seq))
    })
}

fn add_to_index(doc_id: &str, block_id: u64, timestamp: u128, seq: u64) {
    CACHE_INDEX
        .entry((doc_id.to_string(), block_id))
        .or_default()
        .insert(timestamp, seq);
}

fn remove_from_index(doc_id: &str, block_id: u64, timestamp: u128) {
    let index_key = (doc_id.to_string(), block_id);
    if let Some(mut index) = CACHE_INDEX.get_mut(&index_key) {
        index.remove(&timestamp);
        if index.is_empty() {
            drop(index);
            CACHE_INDEX.remove(&index_key);
        }
    }
}

fn set_known_latest(doc_id: &str, block_id: u64, timestamp: u128, seq: u64) {
    KNOWN_LATEST.insert((doc_id.to_string(), block_id), (timestamp, seq));
}

fn get_known_latest(doc_id: &str, block_id: u64) -> Option<(u128, u64)> {
    KNOWN_LATEST
        .get(&(doc_id.to_string(), block_id))
        .map(|v| *v)
}

/// Insert a single block into cache
async fn cache_block(doc_id: &str, block_id: u64, block: &BlockData) {
    let key = (doc_id.to_string(), block_id, block.timestamp);
    BLOCK_CACHE.insert(key, block.clone()).await;
    add_to_index(doc_id, block_id, block.timestamp, block.seq);
}

/// Batch insert multiple blocks into cache
async fn cache_blocks(doc_id: &str, block_id: u64, blocks: Vec<BlockData>) {
    for bd in blocks {
        let ts = bd.timestamp;
        let seq = bd.seq;
        let key = (doc_id.to_string(), block_id, ts);
        BLOCK_CACHE.insert(key, bd).await;
        add_to_index(doc_id, block_id, ts, seq);
    }
}

async fn get_single_block(
    doc_id: String,
    block_id: u64,
    target_ts: u128,
) -> Result<Option<BlockData>, String> {
    // =========================================
    // FAST PATH: Pure in-memory lookups
    // =========================================

    // Check if we have a cached state at or before target_ts
    if let Some((cached_ts, cached_seq)) = find_best_cached(&doc_id, block_id, target_ts) {
        // Check if this IS the latest known state
        if let Some((latest_ts, _)) = get_known_latest(&doc_id, block_id) {
            if cached_ts == latest_ts && target_ts >= latest_ts {
                // We want latest or beyond, and we have latest cached
                let cache_key = (doc_id.clone(), block_id, cached_ts);
                if let Some(cached) = BLOCK_CACHE.get(&cache_key).await {
                    return Ok(Some(cached));
                }
                // Evicted, clear and fall through
                remove_from_index(&doc_id, block_id, cached_ts);
                KNOWN_LATEST.remove(&(doc_id.clone(), block_id));
            }
        }

        // Check if there's a NEXT cached state after target_ts
        if let Some((next_ts, next_seq)) = find_next_cached(&doc_id, block_id, target_ts) {
            // If consecutive (next_seq == cached_seq + 1), no patches exist between them
            // So the state at target_ts IS the state at cached_ts
            if next_seq == cached_seq + 1 {
                let cache_key = (doc_id.clone(), block_id, cached_ts);
                if let Some(cached) = BLOCK_CACHE.get(&cache_key).await {
                    return Ok(Some(cached));
                }
                // Evicted, remove stale index and fall through
                remove_from_index(&doc_id, block_id, cached_ts);
            }
            // Not consecutive - there are patches between, need DB query
        } else {
            // No next cached state - check if cached_ts is the known latest
            if let Some((latest_ts, _)) = get_known_latest(&doc_id, block_id) {
                if cached_ts == latest_ts {
                    let cache_key = (doc_id.clone(), block_id, cached_ts);
                    if let Some(cached) = BLOCK_CACHE.get(&cache_key).await {
                        return Ok(Some(cached));
                    }
                    remove_from_index(&doc_id, block_id, cached_ts);
                    KNOWN_LATEST.remove(&(doc_id.clone(), block_id));
                }
            }
            // Not known latest - might be more patches after, need DB query
        }
    }

    // =========================================
    // SLOW PATH: Need DB queries
    // =========================================
    let initial_ts =
        database::get_latest_initial_timestamp(doc_id.clone(), block_id, target_ts).await?;

    // Try to find useful cached state with retry on eviction
    loop {
        let cached = find_best_cached(&doc_id, block_id, target_ts);

        match (cached, initial_ts) {
            // Exact cache hit (timestamp matches exactly)
            (Some((cts, _)), _) if cts == target_ts => {
                let cache_key = (doc_id.clone(), block_id, cts);
                if let Some(cached) = BLOCK_CACHE.get(&cache_key).await {
                    return Ok(Some(cached));
                }
                remove_from_index(&doc_id, block_id, cts);
                continue;
            }

            // Cache exists and is at or after latest initial
            (Some((cts, _)), Some(its)) if cts >= its => {
                let cache_key = (doc_id.clone(), block_id, cts);
                if let Some(cached) = BLOCK_CACHE.get(&cache_key).await {
                    return reconstruct_from_cached(doc_id, block_id, target_ts, cached, cts).await;
                }
                remove_from_index(&doc_id, block_id, cts);
                continue;
            }

            // Cache exists but initial is newer - use initial
            (Some(_), Some(_)) => break,

            // No useful cache
            _ => break,
        }
    }

    // Reconstruct from initial
    if initial_ts.is_some() {
        reconstruct_from_initial(doc_id, block_id, target_ts).await
    } else {
        Ok(None)
    }
}

async fn reconstruct_from_cached(
    doc_id: String,
    block_id: u64,
    target_ts: u128,
    cached_block: BlockData,
    cached_ts: u128,
) -> Result<Option<BlockData>, String> {
    let batches: Vec<DecryptedBatch> =
        database::get_block_batches_in_range(doc_id.clone(), block_id, cached_ts, target_ts)
            .await?;

    if batches.is_empty() {
        // No batches after cached = cached IS the latest
        if target_ts == u128::MAX {
            set_known_latest(&doc_id, block_id, cached_ts, cached_block.seq);
        }
        return Ok(Some(cached_block));
    }

    let is_query_to_end = target_ts == u128::MAX;
    let starting_seq = cached_block.seq;

    let (result_block, new_states) = tokio::task::spawn_blocking(move || {
        let mut current_block_data = cached_block;
        let mut new_states: Vec<BlockData> = Vec::new();
        let mut current_seq = starting_seq;

        for batch in batches {
            let mut current_time = batch.timestamp;
            let mut is_first_patch_in_batch = true;

            for patch in batch.batch_data.patches.iter() {
                let is_first = is_first_patch_in_batch;
                is_first_patch_in_batch = false;

                current_time += (patch.time_delta * 5) as u128;

                if current_time <= cached_ts {
                    continue;
                }

                if current_time > target_ts {
                    break;
                }

                let (new_block, _) = if batch.is_initial && is_first {
                    decode_initial_patch(patch)?
                } else {
                    decode_patch(&current_block_data.block, patch)?
                };

                current_seq += 1;
                let block_data = BlockData {
                    timestamp: current_time,
                    block_id,
                    block: new_block,
                    seq: current_seq,
                };

                new_states.push(block_data.clone());
                current_block_data = block_data;
            }
        }

        Ok::<_, String>((current_block_data, new_states))
    })
    .await
    .map_err(|e| e.to_string())??;

    if !new_states.is_empty() {
        cache_blocks(&doc_id, block_id, new_states).await;
    }

    if is_query_to_end {
        set_known_latest(&doc_id, block_id, result_block.timestamp, result_block.seq);
    }

    Ok(Some(result_block))
}

async fn reconstruct_from_initial(
    doc_id: String,
    block_id: u64,
    target_ts: u128,
) -> Result<Option<BlockData>, String> {
    let batches: Vec<DecryptedBatch> =
        database::get_block_batches_from_latest_initial_up_to_timestamp(
            doc_id.clone(),
            block_id,
            target_ts,
        )
        .await?;

    if batches.is_empty() {
        return Ok(None);
    }

    let is_query_to_end = target_ts == u128::MAX;
    let doc_id_clone = doc_id.clone();

    let (current_block_data, intermediate_states, snapshots_to_save) =
        tokio::task::spawn_blocking(move || {
            let mut current_block_data: Option<BlockData> = None;
            let mut intermediate_states: Vec<BlockData> = Vec::new();
            let mut snapshots_to_save: Vec<DecryptedBatch> = Vec::new();
            let mut batches_since_snapshot: usize = 0;
            let mut current_seq: u64 = 0;

            for batch in batches {
                let mut current_time = batch.timestamp;
                let mut is_first_patch_in_batch = true;
                let mut first_patch_result: Option<(Block, u8)> = None;

                for (patch_idx, patch) in batch.batch_data.patches.iter().enumerate() {
                    let (new_block, time_delta) = if batch.is_initial && is_first_patch_in_batch {
                        is_first_patch_in_batch = false;
                        decode_initial_patch(patch)?
                    } else if let Some(ref cbd) = current_block_data {
                        decode_patch(&cbd.block, patch)?
                    } else {
                        decode_initial_patch(patch)?
                    };

                    current_time += (time_delta * 5) as u128;

                    if current_time > target_ts {
                        break;
                    }

                    current_seq += 1;
                    let block_data = BlockData {
                        timestamp: current_time,
                        block_id: batch.batch_data.block_id,
                        block: new_block.clone(),
                        seq: current_seq,
                    };

                    intermediate_states.push(block_data.clone());
                    current_block_data = Some(block_data);

                    if patch_idx == 0 {
                        first_patch_result = Some((new_block, time_delta));
                    }
                }

                // Snapshot logic
                if batch.is_initial {
                    batches_since_snapshot = 0;
                } else {
                    batches_since_snapshot += 1;

                    if batches_since_snapshot >= SNAPSHOT_INTERVAL {
                        if let Some((block, time_delta)) = first_patch_result {
                            let initial_patch = encode_initial_patch(0, time_delta, &block)?;
                            let mut snapshot_patches = vec![initial_patch];
                            snapshot_patches
                                .extend(batch.batch_data.patches.iter().skip(1).cloned());

                            let snapshot = DecryptedBatch {
                                batch_id: format!(
                                    "snapshot:{}:{}:{}",
                                    doc_id_clone, batch.batch_data.block_id, batch.timestamp
                                ),
                                doc_id: doc_id_clone.clone(),
                                timestamp: batch.timestamp,
                                batch_data: BatchData {
                                    block_id: batch.batch_data.block_id,
                                    patches: snapshot_patches,
                                },
                                is_initial: true,
                            };
                            snapshots_to_save.push(snapshot);
                            batches_since_snapshot = 0;
                        }
                    }
                }
            }

            Ok::<_, String>((current_block_data, intermediate_states, snapshots_to_save))
        })
        .await
        .map_err(|e| e.to_string())??;

    if !snapshots_to_save.is_empty() {
        tokio::spawn(async move {
            for snapshot in snapshots_to_save {
                if let Err(e) = database::save_snapshot(snapshot).await {
                    eprintln!("Failed to save snapshot: {}", e);
                }
            }
        });
    }

    if !intermediate_states.is_empty() {
        cache_blocks(&doc_id, block_id, intermediate_states).await;
    }

    if is_query_to_end {
        if let Some(ref bd) = current_block_data {
            set_known_latest(&doc_id, block_id, bd.timestamp, bd.seq);
        }
    }

    Ok(current_block_data)
}

pub async fn get_blocks(
    doc_id: String,
    block_ids: Vec<u64>,
    timestamp: u128,
) -> Result<DocumentData, String> {
    let futures: Vec<_> = block_ids
        .into_iter()
        .map(|block_id| get_single_block(doc_id.clone(), block_id, timestamp))
        .collect();

    let results = join_all(futures).await;

    let mut blocks = Vec::with_capacity(results.len());
    for result in results {
        if let Some(block) = result? {
            blocks.push(block);
        }
    }

    // Re-insert all final states to ensure they're freshest in LRU
    // This prevents early blocks from being evicted by later blocks during parallel reconstruction
    if timestamp == u128::MAX {
        for block in &blocks {
            cache_block(&doc_id, block.block_id, block).await;
            set_known_latest(&doc_id, block.block_id, block.timestamp, block.seq);
        }
    } else {
        for block in &blocks {
            cache_block(&doc_id, block.block_id, block).await;
        }
    }

    Ok(DocumentData { doc_id, blocks })
}

pub fn invalidate_block_cache(doc_id: String, block_id: u64) {
    let doc_id_clone = doc_id.clone();
    let _ =
        BLOCK_CACHE.invalidate_entries_if(move |key, _| key.0 == doc_id_clone && key.1 == block_id);
    let index_key = (doc_id.clone(), block_id);
    CACHE_INDEX.remove(&index_key);
    KNOWN_LATEST.remove(&index_key);
}

pub fn invalidate_doc_cache(doc_id: String) {
    let doc_id_clone = doc_id.clone();
    let doc_id_clone2 = doc_id.clone();
    let _ = BLOCK_CACHE.invalidate_entries_if(move |key, _| key.0 == doc_id_clone);
    CACHE_INDEX.retain(|key, _| key.0 != doc_id);
    KNOWN_LATEST.retain(|key, _| key.0 != doc_id_clone2);
}
