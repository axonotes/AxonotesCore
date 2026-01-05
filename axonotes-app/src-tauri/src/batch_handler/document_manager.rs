use crate::batch_handler::block_type_helpers::{decode_initial_patch, decode_patch};
use crate::batch_handler::block_types::Block;
use crate::database;
use crate::encryption::batch::DecryptedBatch;
use futures::future::join_all;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockData {
    timestamp: u128,
    block_id: u64,
    block: Block,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocumentData {
    doc_id: String,
    blocks: Vec<BlockData>,
}

type CacheKey = (String, u64, u128); // (doc_id, block_id, timestamp)

static BLOCK_CACHE: LazyLock<Cache<CacheKey, BlockData>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(10_000)
        .time_to_idle(std::time::Duration::from_secs(300)) // evict after 5min of no access
        .support_invalidation_closures()
        .build()
});

async fn get_single_block(
    doc_id: String,
    block_id: u64,
    timestamp: u128,
) -> Result<Option<BlockData>, String> {
    let cache_key = (doc_id.clone(), block_id, timestamp);

    if let Some(cached) = BLOCK_CACHE.get(&cache_key).await {
        return Ok(Some(cached));
    }

    let batches: Vec<DecryptedBatch> =
        database::get_block_batches_from_latest_initial_up_to_timestamp(
            doc_id, block_id, timestamp,
        )
        .await?;

    // Move CPU-bound patch processing to blocking thread pool
    let current_block_data = tokio::task::spawn_blocking(move || {
        let mut current_block_data: Option<BlockData> = None;

        for batch in batches {
            let mut current_time = batch.timestamp;
            let mut is_first_patch_in_batch = true;

            for patch in batch.batch_data.patches {
                let (new_block, time_delta) = if batch.is_initial && is_first_patch_in_batch {
                    // First patch of an initial batch - decode from scratch
                    is_first_patch_in_batch = false;
                    decode_initial_patch(&patch)?
                } else if let Some(ref cbd) = current_block_data {
                    // Regular patch - decode as delta from previous state
                    decode_patch(&cbd.block, &patch)?
                } else {
                    decode_initial_patch(&patch)?
                };

                current_time += (time_delta * 5) as u128;
                current_block_data = Some(BlockData {
                    timestamp: current_time,
                    block_id: batch.batch_data.block_id,
                    block: new_block,
                });
            }
        }

        Ok::<_, String>(current_block_data)
    })
    .await
    .map_err(|e| e.to_string())??;

    if let Some(ref block_data) = current_block_data {
        BLOCK_CACHE.insert(cache_key, block_data.clone()).await;
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

    Ok(DocumentData { doc_id, blocks })
}

// Call this when batches are added/modified to invalidate stale cache entries
pub fn invalidate_block_cache(doc_id: String, block_id: u64) {
    BLOCK_CACHE
        .invalidate_entries_if(move |(key_doc, key_block, _), _| {
            *key_doc == doc_id && *key_block == block_id
        })
        .unwrap();
}

pub fn invalidate_doc_cache(doc_id: String) {
    BLOCK_CACHE
        .invalidate_entries_if(move |(key_doc, _, _), _| *key_doc == doc_id)
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use crate::encryption::batch::{BatchData, DecryptedBatch, Patch};
    use std::time::Instant;

    fn make_test_batch(
        doc_id: &str,
        block_id: u64,
        batch_num: u32,
        timestamp: u128,
        is_initial: bool,
        patches: Vec<Patch>,
    ) -> DecryptedBatch {
        DecryptedBatch {
            batch_id: format!("batch_{}_{}_{}", doc_id, block_id, batch_num),
            doc_id: doc_id.to_string(),
            timestamp,
            is_initial,
            batch_data: BatchData { block_id, patches },
        }
    }

    fn make_initial_patch(block: &Block) -> Patch {
        crate::batch_handler::block_type_helpers::encode_initial_patch(0, 1, block)
            .expect("Failed to encode initial patch")
    }

    fn make_patch(base: &Block, new: &Block) -> Patch {
        crate::batch_handler::block_type_helpers::encode_patch(0, 1, base, new)
            .expect("Failed to encode patch")
    }

    fn make_test_block(content: &str) -> Block {
        use crate::batch_handler::block_types::{BlockContent, ParagraphV1};
        use spacetimedb_sdk::Identity;

        Block::new(
            1,
            1234567890,
            BlockContent::ParagraphV1(ParagraphV1 {
                deleted: None,
                author: Identity::from_byte_array([0u8; 32]),
                group_id: "main".to_string(),
                group_row: "0".to_string(),
                text: content.to_string(),
                formatting: vec![],
            }),
        )
    }

    async fn setup_test_db() -> Result<String, String> {
        let temp_dir =
            std::env::temp_dir().join(format!("test_db_{}", uuid::Uuid::new_v4()).as_str());
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let _ = database::init_paths(temp_dir);
        if !database::is_unlocked().await {
            database::unlock_db("".to_string()).await?;
        }
        Ok(format!("test_doc_{}", uuid::Uuid::new_v4()))
    }

    async fn cleanup_test_batches(doc_id: &str) {
        if let Ok(batches) = database::get_batches_by_doc(doc_id.to_string()).await {
            for batch in &batches {
                let _ = database::delete_batch(batch).await;
            }
        }
        invalidate_doc_cache(doc_id.to_string());
    }

    #[tokio::test]
    async fn test_get_blocks_single_batch() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block = make_test_block("hello world");
        let patch = make_initial_patch(&block);
        let batch = make_test_batch(doc_id, 1, 0, 100, true, vec![patch]);

        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        let result = get_blocks(doc_id.to_string(), vec![1], u128::MAX).await;
        assert!(result.is_ok(), "get_blocks failed: {:?}", result.err());

        let doc_data = result.unwrap();
        assert_eq!(doc_data.doc_id, doc_id);
        assert_eq!(doc_data.blocks.len(), 1);
        assert_eq!(doc_data.blocks[0].block_id, 1);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_get_blocks_multiple_patches_in_initial_batch() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block_v1 = make_test_block("version 1");
        let block_v2 = make_test_block("version 2");
        let block_v3 = make_test_block("version 3");

        // Initial batch with multiple patches - only first is decoded as initial
        let patches = vec![
            make_initial_patch(&block_v1),
            make_patch(&block_v1, &block_v2),
            make_patch(&block_v2, &block_v3),
        ];

        let batch = make_test_batch(doc_id, 1, 0, 100, true, patches);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        let result = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();
        assert_eq!(result.blocks.len(), 1);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_get_blocks_initial_then_regular_batches() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block_v1 = make_test_block("version 1");
        let block_v2 = make_test_block("version 2");
        let block_v3 = make_test_block("version 3");

        // Initial batch
        let batch1 = make_test_batch(doc_id, 1, 0, 100, true, vec![make_initial_patch(&block_v1)]);
        database::save_batch(batch1)
            .await
            .expect("Failed to save batch");

        // Regular batch (not initial)
        let batch2 = make_test_batch(
            doc_id,
            1,
            1,
            200,
            false,
            vec![make_patch(&block_v1, &block_v2)],
        );
        database::save_batch(batch2)
            .await
            .expect("Failed to save batch");

        // Another regular batch
        let batch3 = make_test_batch(
            doc_id,
            1,
            2,
            300,
            false,
            vec![make_patch(&block_v2, &block_v3)],
        );
        database::save_batch(batch3)
            .await
            .expect("Failed to save batch");

        let result = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();
        assert_eq!(result.blocks.len(), 1);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_get_blocks_parallel() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        for block_id in 1..=5 {
            let block = make_test_block(format!("block {}", block_id).as_str());
            let patch = make_initial_patch(&block);
            let batch = make_test_batch(doc_id, block_id, 0, 100, true, vec![patch]);
            database::save_batch(batch)
                .await
                .expect("Failed to save batch");
        }

        let result = get_blocks(doc_id.to_string(), vec![1, 2, 3, 4, 5], u128::MAX)
            .await
            .unwrap();

        assert_eq!(result.blocks.len(), 5);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block = make_test_block("cached content");
        let patch = make_initial_patch(&block);
        let batch = make_test_batch(doc_id, 1, 0, 100, true, vec![patch]);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        // Warm up - discard this timing
        let _ = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();

        // Now measure cache hits with multiple iterations
        let iterations = 100;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
                .await
                .unwrap();
        }
        let avg_cached = start.elapsed() / iterations;

        println!("Avg cached lookup: {:?}", avg_cached);
        // Just assert it's reasonably fast, don't compare to cold
        assert!(avg_cached < std::time::Duration::from_millis(1));

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block_v1 = make_test_block("original");
        let batch = make_test_batch(doc_id, 1, 0, 100, true, vec![make_initial_patch(&block_v1)]);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        let _ = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();

        let block_v2 = make_test_block("updated");
        let batch2 = make_test_batch(
            doc_id,
            1,
            1,
            200,
            false,
            vec![make_patch(&block_v1, &block_v2)],
        );
        database::save_batch(batch2)
            .await
            .expect("Failed to save batch");

        let result = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();
        assert_eq!(result.blocks.len(), 1);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_timestamp_filtering() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block_v1 = make_test_block("old");
        let block_v2 = make_test_block("new");

        let batch1 = make_test_batch(doc_id, 1, 0, 100, true, vec![make_initial_patch(&block_v1)]);
        database::save_batch(batch1)
            .await
            .expect("Failed to save batch");

        let batch2 = make_test_batch(
            doc_id,
            1,
            1,
            200,
            false,
            vec![make_patch(&block_v1, &block_v2)],
        );
        database::save_batch(batch2)
            .await
            .expect("Failed to save batch");

        // Request at timestamp 150 - should get block_v1 only
        let result = get_blocks(doc_id.to_string(), vec![1], 150).await.unwrap();
        assert_eq!(result.blocks.len(), 1);

        // Request at timestamp 250 - should get block_v2
        let result = get_blocks(doc_id.to_string(), vec![1], 250).await.unwrap();
        assert_eq!(result.blocks.len(), 1);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_nonexistent_block() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let result = get_blocks(doc_id.to_string(), vec![999], u128::MAX)
            .await
            .unwrap();
        assert!(result.blocks.is_empty());
    }

    // ==========================================
    // Benchmarks
    // ==========================================

    #[tokio::test]
    async fn bench_reconstruction_small() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;
        bench_reconstruction(doc_id, 1, 10).await;
    }

    #[tokio::test]
    async fn bench_reconstruction_medium() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;
        bench_reconstruction(doc_id, 1, 100).await;
    }

    #[tokio::test]
    async fn bench_reconstruction_large() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;
        bench_reconstruction(doc_id, 1, 1000).await;
    }

    #[tokio::test]
    async fn bench_parallel_blocks() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;
        let num_blocks = 20;
        let patches_per_block = 50;

        for block_id in 1..=num_blocks {
            let mut patches = vec![];
            let mut current_block = make_test_block(format!("block {} v0", block_id).as_str());
            patches.push(make_initial_patch(&current_block));

            for v in 1..patches_per_block {
                let new_block = make_test_block(format!("block {} v{}", block_id, v).as_str());
                patches.push(make_patch(&current_block, &new_block));
                current_block = new_block;
            }

            // Initial batch with all patches
            let batch = make_test_batch(doc_id, block_id, 0, 100, true, patches);
            database::save_batch(batch)
                .await
                .expect("Failed to save batch");
        }

        invalidate_doc_cache(doc_id.to_string());

        let block_ids: Vec<u64> = (1..=num_blocks).collect();

        let start = Instant::now();
        let result = get_blocks(doc_id.to_string(), block_ids.clone(), u128::MAX)
            .await
            .unwrap();
        let duration = start.elapsed();

        println!("\n=== Parallel Blocks Benchmark ===");
        println!(
            "Blocks: {}, Patches per block: {}",
            num_blocks, patches_per_block
        );
        println!("Total patches: {}", num_blocks * patches_per_block);
        println!("Time: {:?}", duration);
        println!("Blocks retrieved: {}", result.blocks.len());
        println!("Per block: {:?}", duration / num_blocks as u32);

        cleanup_test_batches(doc_id).await;
    }

    async fn bench_reconstruction(doc_id: &str, block_id: u64, num_patches: usize) {
        let mut patches = vec![];
        let mut current_block = make_test_block("v0");
        patches.push(make_initial_patch(&current_block));

        for i in 1..num_patches {
            let new_block = make_test_block(format!("v{}", i).as_str());
            patches.push(make_patch(&current_block, &new_block));
            current_block = new_block;
        }

        let batch = make_test_batch(doc_id, block_id, 0, 100, true, patches);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        invalidate_doc_cache(doc_id.to_string());

        let iterations = 10;
        let mut cold_total = std::time::Duration::ZERO;

        for _ in 0..iterations {
            invalidate_doc_cache(doc_id.to_string());
            let start = Instant::now();
            let _ = get_blocks(doc_id.to_string(), vec![block_id], u128::MAX)
                .await
                .unwrap();
            cold_total += start.elapsed();
        }

        let _ = get_blocks(doc_id.to_string(), vec![block_id], u128::MAX)
            .await
            .unwrap();

        let mut warm_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = get_blocks(doc_id.to_string(), vec![block_id], u128::MAX)
                .await
                .unwrap();
            warm_total += start.elapsed();
        }

        println!(
            "\n=== Reconstruction Benchmark: {} patches ===",
            num_patches
        );
        println!("Cold avg: {:?}", cold_total / iterations as u32);
        println!("Warm avg: {:?}", warm_total / iterations as u32);
        println!(
            "Cache speedup: {:.1}x",
            cold_total.as_nanos() as f64 / warm_total.as_nanos() as f64
        );

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn bench_with_vs_without_periodic_initial_batches() {
        let doc_id_single = setup_test_db().await.expect("Failed to setup test db");
        let doc_id_single = doc_id_single.as_str();
        cleanup_test_batches(doc_id_single).await;

        let doc_id_periodic = format!("{}_periodic", doc_id_single);
        let doc_id_periodic = doc_id_periodic.as_str();
        cleanup_test_batches(doc_id_periodic).await;

        let total_batches = 1000;
        let initial_every = 100;

        // === Scenario A: Single initial batch at the start ===
        {
            let mut current_block = make_test_block("v0");
            let batch = make_test_batch(
                doc_id_single,
                1,
                0,
                0,
                true,
                vec![make_initial_patch(&current_block)],
            );
            database::save_batch(batch).await.expect("Failed");

            for i in 1..total_batches {
                let new_block = make_test_block(format!("v{}", i).as_str());
                let batch = make_test_batch(
                    doc_id_single,
                    1,
                    i as u32,
                    i as u128 * 10,
                    false,
                    vec![make_patch(&current_block, &new_block)],
                );
                database::save_batch(batch).await.expect("Failed");
                current_block = new_block;
            }
        }

        // === Scenario B: Initial batch every 100 batches ===
        {
            let mut current_block = make_test_block("v0");

            for i in 0..total_batches {
                let is_initial = i % initial_every == 0;
                let new_block = make_test_block(format!("v{}", i).as_str());

                let patch = if is_initial {
                    make_initial_patch(&new_block)
                } else {
                    make_patch(&current_block, &new_block)
                };

                let batch = make_test_batch(
                    doc_id_periodic,
                    1,
                    i as u32,
                    i as u128 * 10,
                    is_initial,
                    vec![patch],
                );
                database::save_batch(batch).await.expect("Failed");
                current_block = new_block;
            }
        }

        // Query near the end (timestamp that includes ~990 batches)
        let query_timestamp = (total_batches - 10) as u128 * 10;

        // Benchmark single initial
        invalidate_doc_cache(doc_id_single.to_string());
        let iterations = 10;

        let mut single_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            invalidate_doc_cache(doc_id_single.to_string());
            let start = Instant::now();
            let result = get_blocks(doc_id_single.to_string(), vec![1], query_timestamp)
                .await
                .unwrap();
            single_total += start.elapsed();
            assert_eq!(result.blocks.len(), 1);
        }

        // Benchmark periodic initial
        let mut periodic_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            invalidate_doc_cache(doc_id_periodic.to_string());
            let start = Instant::now();
            let result = get_blocks(doc_id_periodic.to_string(), vec![1], query_timestamp)
                .await
                .unwrap();
            periodic_total += start.elapsed();
            assert_eq!(result.blocks.len(), 1);
        }

        println!("\n=== Initial Batch Strategy Benchmark ===");
        println!("Total batches: {}", total_batches);
        println!("Query timestamp: {} (near end)", query_timestamp);
        println!();
        println!(
            "Single initial batch (processes ~{} batches):",
            total_batches - 10
        );
        println!("  Avg: {:?}", single_total / iterations as u32);
        println!();
        println!(
            "Periodic initial batches (processes ~{} batches):",
            initial_every - 10
        );
        println!("  Avg: {:?}", periodic_total / iterations as u32);
        println!();
        println!(
            "Speedup from periodic initials: {:.1}x",
            single_total.as_nanos() as f64 / periodic_total.as_nanos() as f64
        );

        cleanup_test_batches(doc_id_single).await;
        cleanup_test_batches(doc_id_periodic).await;
    }

    #[tokio::test]
    #[ignore]
    async fn bench_reconstruction_stress() {
        use rayon::prelude::*;

        let doc_id_single = setup_test_db().await.expect("Failed to setup test db");
        let doc_id_single = doc_id_single.as_str();
        cleanup_test_batches(doc_id_single).await;

        let doc_id_periodic = format!("{}_periodic", doc_id_single);
        let doc_id_periodic = doc_id_periodic.as_str();
        cleanup_test_batches(doc_id_periodic).await;

        let num_blocks: u64 = 100;
        let patches_per_block: usize = 100_000;
        let initial_every: usize = 100;

        println!("\n=== Stress Test Setup ===");
        println!(
            "Creating {} blocks with {} patches each...",
            num_blocks, patches_per_block
        );
        println!(
            "Total patches per scenario: {}",
            num_blocks as usize * patches_per_block
        );
        println!(
            "Initial batch every {} patches (periodic scenario)",
            initial_every
        );

        // === Scenario A: Single initial batch at the start ===
        println!("\n--- Building single-initial batches ---");
        let setup_start = Instant::now();

        let doc_id_clone = doc_id_single.to_string();
        let batches_single: Vec<_> = (1..=num_blocks)
            .into_par_iter()
            .map(|block_id| {
                let mut patches = Vec::with_capacity(patches_per_block);
                let mut current_block = make_test_block(format!("block {} v0", block_id).as_str());
                patches.push(make_initial_patch(&current_block));

                for v in 1..patches_per_block {
                    let new_block = make_test_block(format!("block {} v{}", block_id, v).as_str());
                    patches.push(make_patch(&current_block, &new_block));
                    current_block = new_block;
                }

                // Single initial batch containing all patches
                make_test_batch(doc_id_clone.as_str(), block_id, 0, 100, true, patches)
            })
            .collect();

        println!(
            "Single-initial batches created in {:?}",
            setup_start.elapsed()
        );

        // === Scenario B: Periodic initial batches ===
        println!("\n--- Building periodic-initial batches ---");
        let setup_start_periodic = Instant::now();

        let doc_id_clone = doc_id_periodic.to_string();
        let batches_periodic: Vec<_> = (1..=num_blocks)
            .into_par_iter()
            .flat_map(|block_id| {
                let mut batches = Vec::new();
                let mut current_block = make_test_block(format!("block {} v0", block_id).as_str());

                for batch_num in 0..(patches_per_block / initial_every) {
                    let start_patch = batch_num * initial_every;
                    let is_initial = true; // Each batch starts with an initial

                    let mut patches = Vec::with_capacity(initial_every);

                    for v in start_patch..(start_patch + initial_every) {
                        let new_block =
                            make_test_block(format!("block {} v{}", block_id, v).as_str());

                        let patch = if v == start_patch {
                            // First patch in batch is initial
                            make_initial_patch(&new_block)
                        } else {
                            make_patch(&current_block, &new_block)
                        };

                        patches.push(patch);
                        current_block = new_block;
                    }

                    let timestamp = (start_patch as u128) * 10;
                    batches.push(make_test_batch(
                        doc_id_clone.as_str(),
                        block_id,
                        batch_num as u32,
                        timestamp,
                        is_initial,
                        patches,
                    ));
                }

                batches
            })
            .collect();

        println!(
            "Periodic-initial batches created in {:?}",
            setup_start_periodic.elapsed()
        );
        println!("Total batches (periodic): {}", batches_periodic.len());

        // === Save all batches ===
        println!("\n--- Saving batches ---");
        let save_start = Instant::now();

        let save_futures: Vec<_> = batches_single
            .into_iter()
            .map(|batch| database::save_batch(batch))
            .collect();
        let results = join_all(save_futures).await;
        for result in results {
            result.expect("Failed to save batch");
        }
        println!("Single-initial batches saved in {:?}", save_start.elapsed());

        let save_start = Instant::now();
        let save_futures: Vec<_> = batches_periodic
            .into_iter()
            .map(|batch| database::save_batch(batch))
            .collect();
        let results = join_all(save_futures).await;
        for result in results {
            result.expect("Failed to save batch");
        }
        println!(
            "Periodic-initial batches saved in {:?}",
            save_start.elapsed()
        );

        // === Benchmark Setup ===
        let block_ids: Vec<u64> = (1..=num_blocks).collect();
        // Query near the end
        let query_timestamp = ((patches_per_block - 10) as u128) * 10;

        // === Benchmark: Single Initial ===
        println!("\n=== Benchmarking Single Initial ===");
        invalidate_doc_cache(doc_id_single.to_string());

        let start = Instant::now();
        let result = get_blocks(
            doc_id_single.to_string(),
            block_ids.clone(),
            query_timestamp,
        )
        .await
        .unwrap();
        let single_cold = start.elapsed();

        println!("Cold reconstruction: {:?}", single_cold);
        println!("Blocks retrieved: {}", result.blocks.len());
        println!("Per block (cold): {:?}", single_cold / num_blocks as u32);

        let start = Instant::now();
        let _ = get_blocks(
            doc_id_single.to_string(),
            block_ids.clone(),
            query_timestamp,
        )
        .await
        .unwrap();
        let single_warm = start.elapsed();

        println!("Warm (cached): {:?}", single_warm);
        println!(
            "Cache speedup: {:.1}x",
            single_cold.as_nanos() as f64 / single_warm.as_nanos() as f64
        );

        // === Benchmark: Periodic Initial ===
        println!(
            "\n=== Benchmarking Periodic Initial (every {}) ===",
            initial_every
        );
        invalidate_doc_cache(doc_id_periodic.to_string());

        let start = Instant::now();
        let result = get_blocks(
            doc_id_periodic.to_string(),
            block_ids.clone(),
            query_timestamp,
        )
        .await
        .unwrap();
        let periodic_cold = start.elapsed();

        println!("Cold reconstruction: {:?}", periodic_cold);
        println!("Blocks retrieved: {}", result.blocks.len());
        println!("Per block (cold): {:?}", periodic_cold / num_blocks as u32);

        let start = Instant::now();
        let _ = get_blocks(
            doc_id_periodic.to_string(),
            block_ids.clone(),
            query_timestamp,
        )
        .await
        .unwrap();
        let periodic_warm = start.elapsed();

        println!("Warm (cached): {:?}", periodic_warm);
        println!(
            "Cache speedup: {:.1}x",
            periodic_cold.as_nanos() as f64 / periodic_warm.as_nanos() as f64
        );

        // === Summary ===
        println!("\n========================================");
        println!("=== SUMMARY ===");
        println!("========================================");
        println!(
            "Blocks: {}, Patches per block: {}",
            num_blocks, patches_per_block
        );
        println!("Query timestamp: {} (near end)", query_timestamp);
        println!();
        println!(
            "Single initial (processes all {} patches per block):",
            patches_per_block
        );
        println!("  Cold: {:?}", single_cold);
        println!("  Warm: {:?}", single_warm);
        println!();
        println!(
            "Periodic initial every {} (processes ~{} patches per block):",
            initial_every, initial_every
        );
        println!("  Cold: {:?}", periodic_cold);
        println!("  Warm: {:?}", periodic_warm);
        println!();
        println!(
            "Speedup from periodic initials (cold): {:.1}x",
            single_cold.as_nanos() as f64 / periodic_cold.as_nanos() as f64
        );
        println!(
            "Speedup from periodic initials (warm): {:.1}x",
            single_warm.as_nanos() as f64 / periodic_warm.as_nanos() as f64
        );

        cleanup_test_batches(doc_id_single).await;
        cleanup_test_batches(doc_id_periodic).await;
    }
}
