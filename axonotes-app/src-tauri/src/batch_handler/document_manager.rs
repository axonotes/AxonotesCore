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
        database::get_by_doc_and_block_up_to_timestamp(doc_id, block_id, timestamp).await?;

    // Move CPU-bound patch processing to blocking thread pool
    let current_block_data = tokio::task::spawn_blocking(move || {
        let mut current_block_data: Option<BlockData> = None;

        for batch in batches {
            let mut current_time = batch.timestamp;

            for patch in batch.batch_data.patches {
                let (new_block, time_delta) = if let Some(ref cbd) = current_block_data {
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
        patches: Vec<Patch>,
    ) -> DecryptedBatch {
        DecryptedBatch {
            batch_id: format!("batch_{}_{}_{}", doc_id, block_id, batch_num),
            doc_id: doc_id.to_string(),
            timestamp,
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
        // Return a unique doc_id for this test
        Ok(format!("test_doc_{}", uuid::Uuid::new_v4()))
    }

    async fn cleanup_test_batches(doc_id: &str) {
        // Delete all test batches for this doc
        if let Ok(batches) = database::get_batches_by_doc(doc_id.to_string()).await {
            for batch in &batches {
                let _ = database::delete_batch(batch).await;
            }
        }
        // Also invalidate cache
        invalidate_doc_cache(doc_id.to_string());
    }

    #[tokio::test]
    async fn test_get_blocks_single_batch() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await;

        let block = make_test_block("hello world");
        let patch = make_initial_patch(&block);
        let batch = make_test_batch(doc_id, 1, 0, 100, vec![patch]);

        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        // Use u128::MAX to get "latest" state (all batches up to now)
        let result = get_blocks(doc_id.to_string(), vec![1], u128::MAX).await;
        assert!(result.is_ok(), "get_blocks failed: {:?}", result.err());

        let doc_data = result.unwrap();
        assert_eq!(doc_data.doc_id, doc_id);
        assert_eq!(doc_data.blocks.len(), 1);
        assert_eq!(doc_data.blocks[0].block_id, 1);

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_get_blocks_multiple_patches() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first

        let block_v1 = make_test_block("version 1");
        let block_v2 = make_test_block("version 2");
        let block_v3 = make_test_block("version 3");

        let patches = vec![
            make_initial_patch(&block_v1),
            make_patch(&block_v1, &block_v2),
            make_patch(&block_v2, &block_v3),
        ];

        let batch = make_test_batch(doc_id, 1, 0, 100, patches);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        let result = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();
        assert_eq!(result.blocks.len(), 1);
        // Final block should be v3
        // Add assertions based on your Block structure

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_get_blocks_parallel() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first

        // Create 5 different blocks
        for block_id in 1..=5 {
            let block = make_test_block(format!("block {}", block_id).as_str());
            let patch = make_initial_patch(&block);
            let batch = make_test_batch(doc_id, block_id, 0, 100, vec![patch]);
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
        cleanup_test_batches(doc_id).await; // Clean first

        let block = make_test_block("cached content");
        let patch = make_initial_patch(&block);
        let batch = make_test_batch(doc_id, 1, 0, 100, vec![patch]);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        // First call - cache miss
        let start = Instant::now();
        let _ = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();
        let first_duration = start.elapsed();

        // Second call - should be cache hit (much faster)
        let start = Instant::now();
        let _ = get_blocks(doc_id.to_string(), vec![1], u128::MAX)
            .await
            .unwrap();
        let second_duration = start.elapsed();

        println!("First call (cache miss): {:?}", first_duration);
        println!("Second call (cache hit): {:?}", second_duration);

        // Cache hit should be significantly faster
        assert!(
            second_duration < first_duration / 2,
            "Cache hit should be at least 2x faster"
        );

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first

        let block_v1 = make_test_block("original");
        let patch = make_initial_patch(&block_v1);
        let batch = make_test_batch(doc_id, 1, 0, 100, vec![patch]);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        // Populate cache
        let _ = get_blocks(doc_id.to_string(), vec![1], 0).await.unwrap();

        // Add new batch (this should invalidate via save_batch)
        let block_v2 = make_test_block("updated");
        let patch2 = make_patch(&block_v1, &block_v2);
        let batch2 = make_test_batch(doc_id, 1, 1, 200, vec![patch2]);
        database::save_batch(batch2)
            .await
            .expect("Failed to save batch");

        // Should get updated data, not cached
        let result = get_blocks(doc_id.to_string(), vec![1], 0).await.unwrap();
        // Assert the block reflects v2, not v1
        // Add assertions based on your Block structure

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_timestamp_filtering() {
        setup_test_db().await.expect("Failed to setup test db");
        let doc_id = "test_doc_timestamp";
        cleanup_test_batches(doc_id).await;

        let block_v1 = make_test_block("old");
        let block_v2 = make_test_block("new");

        // Batch at timestamp 100
        let batch1 = make_test_batch(doc_id, 1, 0, 100, vec![make_initial_patch(&block_v1)]);
        database::save_batch(batch1)
            .await
            .expect("Failed to save batch");

        // Batch at timestamp 200
        let batch2 = make_test_batch(doc_id, 1, 1, 200, vec![make_patch(&block_v1, &block_v2)]);
        database::save_batch(batch2)
            .await
            .expect("Failed to save batch");

        // Request at timestamp 150 - should get block_v1 (only batch1 applies)
        let result = get_blocks(doc_id.to_string(), vec![1], 150).await.unwrap();
        assert_eq!(result.blocks.len(), 1);
        // Block should have "old" content, not "new"

        // Request at timestamp 250 - should get block_v2 (both batches apply)
        let result = get_blocks(doc_id.to_string(), vec![1], 250).await.unwrap();
        assert_eq!(result.blocks.len(), 1);
        // Block should have "new" content

        cleanup_test_batches(doc_id).await;
    }

    #[tokio::test]
    async fn test_nonexistent_block() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first

        let result = get_blocks(doc_id.to_string(), vec![999], 0).await.unwrap();
        assert!(result.blocks.is_empty());
    }

    // ==========================================
    // Benchmarks
    // ==========================================

    #[tokio::test]
    async fn bench_reconstruction_small() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first
        bench_reconstruction(doc_id, 1, 10).await;
    }

    #[tokio::test]
    async fn bench_reconstruction_medium() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first
        bench_reconstruction(doc_id, 1, 100).await;
    }

    #[tokio::test]
    async fn bench_reconstruction_large() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first
        bench_reconstruction(doc_id, 1, 1000).await;
    }

    #[tokio::test]
    async fn bench_parallel_blocks() {
        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id = doc_id.as_str();
        cleanup_test_batches(doc_id).await; // Clean first
        let num_blocks = 20;
        let patches_per_block = 50;

        // Setup: create multiple blocks with patches
        for block_id in 1..=num_blocks {
            let mut patches = vec![];
            let mut current_block = make_test_block(format!("block {} v0", block_id).as_str());
            patches.push(make_initial_patch(&current_block));

            for v in 1..patches_per_block {
                let new_block = make_test_block(format!("block {} v{}", block_id, v).as_str());
                patches.push(make_patch(&current_block, &new_block));
                current_block = new_block;
            }

            let batch = make_test_batch(doc_id, block_id, 0, 100, patches);
            database::save_batch(batch)
                .await
                .expect("Failed to save batch");
        }

        // Clear cache
        invalidate_doc_cache(doc_id.to_string());

        // Benchmark
        let block_ids: Vec<u64> = (1..=num_blocks).collect();

        let start = Instant::now();
        let result = get_blocks(doc_id.to_string(), block_ids.clone(), 0)
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
        // Setup: create a batch with many patches
        let mut patches = vec![];
        let mut current_block = make_test_block("v0");
        patches.push(make_initial_patch(&current_block));

        for i in 1..num_patches {
            let new_block = make_test_block(format!("v{}", i).as_str());
            patches.push(make_patch(&current_block, &new_block));
            current_block = new_block;
        }

        let batch = make_test_batch(doc_id, block_id, 0, 100, patches);
        database::save_batch(batch)
            .await
            .expect("Failed to save batch");

        // Clear cache to measure cold performance
        invalidate_doc_cache(doc_id.to_string());

        // Benchmark cold (no cache)
        let iterations = 10;
        let mut cold_total = std::time::Duration::ZERO;

        for _ in 0..iterations {
            invalidate_doc_cache(doc_id.to_string());
            let start = Instant::now();
            let _ = get_blocks(doc_id.to_string(), vec![block_id], 0)
                .await
                .unwrap();
            cold_total += start.elapsed();
        }

        // Benchmark warm (with cache)
        let _ = get_blocks(doc_id.to_string(), vec![block_id], 0)
            .await
            .unwrap();

        let mut warm_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = get_blocks(doc_id.to_string(), vec![block_id], 0)
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
    #[ignore] // Run with `cargo test bench_reconstruction_stress -- --ignored --nocapture`
    async fn bench_reconstruction_stress() {
        use rayon::prelude::*;

        let doc_id = setup_test_db().await.expect("Failed to setup test db");
        let doc_id_str = doc_id.as_str();
        cleanup_test_batches(doc_id_str).await;

        let num_blocks: u64 = 100;
        let patches_per_block: usize = 100_000;

        println!("\n=== Stress Test Setup ===");
        println!(
            "Creating {} blocks with {} patches each...",
            num_blocks, patches_per_block
        );
        println!("Total patches: {}", num_blocks as usize * patches_per_block);

        let setup_start = Instant::now();

        // Create all batches in parallel (CPU-bound work)
        let doc_id_clone = doc_id.clone();
        let batches: Vec<_> = (1..=num_blocks)
            .into_par_iter()
            .map(|block_id| {
                let mut patches = Vec::with_capacity(patches_per_block);
                let mut current_block = make_test_block(&format!("block {} v0", block_id));
                patches.push(make_initial_patch(&current_block));

                for v in 1..patches_per_block {
                    let new_block = make_test_block(&format!("block {} v{}", block_id, v));
                    patches.push(make_patch(&current_block, &new_block));
                    current_block = new_block;
                }

                make_test_batch(&doc_id_clone, block_id, 0, 100, patches)
            })
            .collect();

        println!("Batches created in {:?}", setup_start.elapsed());

        // Save all batches in parallel
        let save_start = Instant::now();
        let save_futures: Vec<_> = batches
            .into_iter()
            .map(|batch| database::save_batch(batch))
            .collect();

        let results = join_all(save_futures).await;
        for result in results {
            result.expect("Failed to save batch");
        }

        println!("Batches saved in {:?}", save_start.elapsed());
        println!("Setup completed in {:?}", setup_start.elapsed());

        // ... rest of benchmark unchanged
        invalidate_doc_cache(doc_id.to_string());

        let block_ids: Vec<u64> = (1..=num_blocks).collect();

        println!("\n=== Running Cold Benchmark ===");
        let start = Instant::now();
        let result = get_blocks(doc_id.to_string(), block_ids.clone(), u128::MAX)
            .await
            .unwrap();
        let cold_duration = start.elapsed();

        println!("Cold reconstruction: {:?}", cold_duration);
        println!("Blocks retrieved: {}", result.blocks.len());
        println!("Per block (cold): {:?}", cold_duration / num_blocks as u32);
        println!(
            "Per patch (cold): {:?}",
            cold_duration / (num_blocks as u32 * patches_per_block as u32)
        );

        println!("\n=== Running Warm Benchmark ===");
        let start = Instant::now();
        let _ = get_blocks(doc_id.to_string(), block_ids.clone(), u128::MAX)
            .await
            .unwrap();
        let warm_duration = start.elapsed();

        println!("Warm (cached): {:?}", warm_duration);
        println!("Per block (warm): {:?}", warm_duration / num_blocks as u32);
        println!(
            "Cache speedup: {:.1}x",
            cold_duration.as_nanos() as f64 / warm_duration.as_nanos() as f64
        );

        cleanup_test_batches(doc_id_str).await;
    }
}
