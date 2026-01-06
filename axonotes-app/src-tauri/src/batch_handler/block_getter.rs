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
use std::sync::LazyLock;

const SNAPSHOT_INTERVAL: usize = 100;
const CACHE_SIZE_ENTRIES: u64 = 100_000; // ~100K blocks worth of batches

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockData {
    pub timestamp: u128,
    pub block_id: u64,
    pub block: Block,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocumentData {
    pub doc_id: String,
    pub blocks: Vec<BlockData>,
}

/// Wrapper around DecryptedBatch with sequential index
#[derive(Clone, Debug)]
struct CachedBatch {
    batch: DecryptedBatch,
    seq: u64, // Sequential batch number (not patch number)
}

type CacheKey = (String, u64); // (doc_id, block_id)

/// Cache of batches per block
/// Key: (doc_id, block_id) -> BTreeMap<batch_timestamp, CachedBatch>
static BATCH_CACHE: LazyLock<Cache<CacheKey, BTreeMap<u128, CachedBatch>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(CACHE_SIZE_ENTRIES)
        .time_to_idle(std::time::Duration::from_secs(300))
        .support_invalidation_closures()
        .eviction_listener(|key: std::sync::Arc<CacheKey>, _value, _cause| {
            KNOWN_LATEST.remove(key.as_ref());
        })
        .build()
});

/// Tracks the latest known batch seq per (doc_id, block_id)
static KNOWN_LATEST: LazyLock<DashMap<CacheKey, u64>> = LazyLock::new(DashMap::new);

/// Find best cached batch at or before target_ts
fn find_best_cached_batch(
    cached: &BTreeMap<u128, CachedBatch>,
    target_ts: u128,
) -> Option<&CachedBatch> {
    cached
        .range(..=target_ts)
        .next_back()
        .map(|(_, batch)| batch)
}

/// Find next cached batch after target_ts
fn find_next_cached_batch(
    cached: &BTreeMap<u128, CachedBatch>,
    target_ts: u128,
) -> Option<&CachedBatch> {
    cached
        .range((
            std::ops::Bound::Excluded(target_ts),
            std::ops::Bound::Unbounded,
        ))
        .next()
        .map(|(_, batch)| batch)
}

/// Get batches needed for reconstruction, using cache when possible
async fn get_batches_for_block(
    doc_id: &str,
    block_id: u64,
    target_ts: u128,
) -> Result<Vec<DecryptedBatch>, String> {
    let cache_key = (doc_id.to_string(), block_id);
    let is_query_to_end = target_ts == u128::MAX;

    // Try cache first
    if let Some(cached) = BATCH_CACHE.get(&cache_key).await {
        // Find best batch at or before target_ts
        if let Some(best) = find_best_cached_batch(&cached, target_ts) {
            let known_latest = KNOWN_LATEST.get(&cache_key).map(|v| *v);

            // Case 1: This batch IS the known latest and we want latest
            if let Some(latest_seq) = known_latest {
                if best.seq == latest_seq && target_ts >= best.batch.timestamp {
                    // Return all cached batches up to target
                    let batches: Vec<_> = cached
                        .range(..=target_ts)
                        .map(|(_, cb)| cb.batch.clone())
                        .collect();
                    return Ok(batches);
                }
            }

            // Case 2: Check if there's a next batch and they're consecutive
            if let Some(next) = find_next_cached_batch(&cached, target_ts) {
                if next.seq == best.seq + 1 {
                    // Consecutive! No batches between, use cache
                    let batches: Vec<_> = cached
                        .range(..=target_ts)
                        .map(|(_, cb)| cb.batch.clone())
                        .collect();
                    return Ok(batches);
                }
            } else if let Some(latest_seq) = known_latest {
                // No next batch, but if best IS the latest, use it
                if best.seq == latest_seq {
                    let batches: Vec<_> = cached
                        .range(..=target_ts)
                        .map(|(_, cb)| cb.batch.clone())
                        .collect();
                    return Ok(batches);
                }
            }
        }
    }

    // Cache miss or incomplete - query DB
    let batches = database::get_block_batches_from_latest_initial_up_to_timestamp(
        doc_id.to_string(),
        block_id,
        target_ts,
    )
    .await?;

    // Cache the batches with sequential indices
    if !batches.is_empty() {
        let mut cached_map = BTreeMap::new();
        for (seq, batch) in batches.iter().enumerate() {
            cached_map.insert(
                batch.timestamp,
                CachedBatch {
                    batch: batch.clone(),
                    seq: seq as u64,
                },
            );
        }

        let latest_seq = (batches.len() - 1) as u64;
        BATCH_CACHE.insert(cache_key.clone(), cached_map).await;

        if is_query_to_end {
            KNOWN_LATEST.insert(cache_key, latest_seq);
        }
    }

    Ok(batches)
}

async fn get_single_block(
    doc_id: String,
    block_id: u64,
    target_ts: u128,
) -> Result<Option<BlockData>, String> {
    let batches = get_batches_for_block(&doc_id, block_id, target_ts).await?;

    if batches.is_empty() {
        return Ok(None);
    }

    let is_query_to_end = target_ts == u128::MAX;
    let doc_id_clone = doc_id.clone();

    // Reconstruct from batches (this is fast! ~25µs)
    let (block_data, snapshots_to_save) = tokio::task::spawn_blocking(move || {
        let mut current_block_data: Option<BlockData> = None;
        let mut snapshots_to_save: Vec<DecryptedBatch> = Vec::new();
        let mut batches_since_snapshot: usize = 0;

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

                current_time += u128::from(time_delta * 5);

                if current_time > target_ts {
                    break;
                }

                current_block_data = Some(BlockData {
                    timestamp: current_time,
                    block_id: batch.batch_data.block_id,
                    block: new_block.clone(),
                });

                if patch_idx == 0 {
                    first_patch_result = Some((new_block, time_delta));
                }
            }

            // Snapshot logic (only when querying to end)
            if is_query_to_end {
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
        }

        Ok::<_, String>((current_block_data, snapshots_to_save))
    })
    .await
    .map_err(|e| e.to_string())??;

    // Save snapshots in background
    if !snapshots_to_save.is_empty() {
        tokio::spawn(async move {
            for snapshot in snapshots_to_save {
                if let Err(e) = database::save_snapshot(snapshot).await {
                    eprintln!("Failed to save snapshot: {e}");
                }
            }
        });
    }

    Ok(block_data)
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

    Ok(DocumentData { doc_id, blocks })
}

pub async fn invalidate_block_cache(doc_id: String, block_id: u64) {
    let key = (doc_id, block_id);
    BATCH_CACHE.invalidate(&key).await;
    KNOWN_LATEST.remove(&key);
}

#[allow(dead_code)]
#[allow(clippy::needless_pass_by_value)] // Consistent with other cache invalidation APIs
pub fn invalidate_doc_cache(doc_id: String) {
    let doc_id_clone = doc_id.clone();
    if let Err(e) = BATCH_CACHE.invalidate_entries_if(move |key, _| key.0 == doc_id_clone) {
        eprintln!("Failed to invalidate cache entries: {e}");
    }
    KNOWN_LATEST.retain(|key, _| key.0 != doc_id);
}
