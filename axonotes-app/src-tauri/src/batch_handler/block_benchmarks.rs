#[cfg(test)]
mod benchmarks {
    use crate::batch_handler::block_getter::*;
    use crate::batch_handler::block_types::{Block, BlockContent, ParagraphV1};
    use crate::database;
    use crate::encryption::batch::{BatchData, DecryptedBatch, Patch};
    use spacetimedb_sdk::Identity;
    use std::time::Instant;

    const PATCHES_PER_BATCH: usize = 20;

    fn make_test_block(id: u64, text: &str) -> Block {
        Block::new(
            id,
            1234567890,
            None,
            BlockContent::ParagraphV1(ParagraphV1 {
                author: Identity::from_byte_array([0u8; 32]),
                group_id: "main".to_string(),
                group_row: "0".to_string(),
                text: text.to_string(),
                formatting: vec![],
            }),
        )
    }

    fn make_initial_patch(block: &Block, time_delta: u8) -> Patch {
        crate::batch_handler::block_type_helpers::encode_initial_patch(0, time_delta, block)
            .expect("encode failed")
    }

    fn make_patch(base: &Block, new: &Block, time_delta: u8) -> Patch {
        crate::batch_handler::block_type_helpers::encode_patch(0, time_delta, base, new)
            .expect("encode failed")
    }

    fn make_batch(
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

    /// Creates realistic batches: multiple batches with ~PATCHES_PER_BATCH patches each
    fn create_realistic_batches(
        doc_id: &str,
        block_id: u64,
        total_patches: usize,
        initial_every_n_batches: Option<usize>,
    ) -> Vec<DecryptedBatch> {
        let mut batches = Vec::new();
        let mut current = make_test_block(block_id, &format!("b{}_v0", block_id));
        let mut patch_count = 0;
        let mut batch_num = 0u32;

        while patch_count < total_patches {
            let patches_this_batch = std::cmp::min(PATCHES_PER_BATCH, total_patches - patch_count);
            let mut patches = Vec::with_capacity(patches_this_batch);

            let is_initial = match initial_every_n_batches {
                Some(n) => (batch_num as usize).is_multiple_of(n),
                None => batch_num == 0,
            };

            for i in 0..patches_this_batch {
                let new = make_test_block(block_id, &format!("b{}_v{}", block_id, patch_count + i));

                let patch = if i == 0 && is_initial {
                    make_initial_patch(&new, 1)
                } else {
                    make_patch(&current, &new, 1)
                };

                patches.push(patch);
                current = new;
            }

            let timestamp = (batch_num as u128) * 1000;
            batches.push(make_batch(
                doc_id, block_id, batch_num, timestamp, is_initial, patches,
            ));

            patch_count += patches_this_batch;
            batch_num += 1;
        }

        batches
    }

    async fn setup_bench() -> String {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let temp_dir = std::env::temp_dir().join("axonotes_bench_db");
            std::fs::create_dir_all(&temp_dir).expect("create dir failed");
            let _ = database::init_paths(temp_dir);
        });

        if !database::is_unlocked().await {
            database::unlock_db("".to_string())
                .await
                .expect("unlock failed");
        }

        format!("bench_doc_{}", uuid::Uuid::new_v4())
    }

    async fn cleanup(doc_id: &str) {
        let _ = database::delete_batches_by_doc(doc_id.to_string()).await;
        invalidate_doc_cache(doc_id.to_string());
    }

    #[tokio::test]
    async fn bench_cache_speedup() {
        let doc_id = setup_bench().await;
        cleanup(&doc_id).await;

        // 500 patches = 25 batches of 20 patches each
        let total_patches = 500;
        let batches = create_realistic_batches(&doc_id, 1, total_patches, None);

        println!("\n=== Cache Speedup Benchmark ===");
        println!("Total patches: {}", total_patches);
        println!("Total batches: {}", batches.len());

        for batch in batches {
            database::save_batch(batch).await.expect("save failed");
        }

        // Cold benchmark (no cache, requires DB query)
        let iterations = 5;
        let mut cold_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            invalidate_doc_cache(doc_id.clone());
            let start = Instant::now();
            let _ = get_blocks(doc_id.clone(), vec![1], u128::MAX)
                .await
                .unwrap();
            cold_total += start.elapsed();
        }

        // Warm benchmark (cache hit, no DB query, still reconstructs)
        let _ = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .unwrap();
        let mut warm_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = get_blocks(doc_id.clone(), vec![1], u128::MAX)
                .await
                .unwrap();
            warm_total += start.elapsed();
        }

        let cold_avg = cold_total / iterations as u32;
        let warm_avg = warm_total / iterations as u32;
        let speedup = cold_total.as_nanos() as f64 / warm_total.as_nanos() as f64;

        println!("Cold avg: {:?}", cold_avg);
        println!("Warm avg: {:?}", warm_avg);
        println!("Speedup:  {:.1}x", speedup);

        // With batch caching, warm still reconstructs so speedup is modest
        // Main benefit is avoiding DB query (~10-20ms) vs reconstruction (~1ms)
        assert!(
            warm_avg.as_millis() < 50,
            "Expected warm query under 50ms, got {:?}",
            warm_avg
        );

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn bench_incremental_reconstruction() {
        let doc_id = setup_bench().await;
        cleanup(&doc_id).await;

        // 2000 patches = 100 batches
        let total_patches = 2000;
        let batches = create_realistic_batches(&doc_id, 1, total_patches, None);
        let num_batches = batches.len();

        println!("\n=== Incremental Reconstruction Benchmark ===");
        println!("Total patches: {}", total_patches);
        println!("Total batches: {}", num_batches);

        for batch in batches {
            database::save_batch(batch).await.expect("save failed");
        }

        // Full reconstruction (cold)
        invalidate_doc_cache(doc_id.clone());
        let start = Instant::now();
        let _ = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .unwrap();
        let full_time = start.elapsed();

        // Historical query at midpoint (uses cached batches)
        let midpoint_ts = 50 * 1000 + 500;
        let start = Instant::now();
        let _ = get_blocks(doc_id.clone(), vec![1], midpoint_ts)
            .await
            .unwrap();
        let historical_time = start.elapsed();

        println!("Full reconstruction:        {:?}", full_time);
        println!("Historical (from cache):    {:?}", historical_time);

        // Historical should be similar or faster (reconstructs fewer patches)
        // No strict assertion - just informational

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn bench_periodic_vs_single_initial() {
        let doc_id_single = setup_bench().await;
        cleanup(&doc_id_single).await;

        let doc_id_periodic = format!("{}_periodic", doc_id_single);
        cleanup(&doc_id_periodic).await;

        let total_patches = 10000;
        let initial_every = 50;

        println!("\n=== Periodic Initial Benchmark ===");
        println!("Total patches: {}", total_patches);
        println!("Patches per batch: {}", PATCHES_PER_BATCH);
        println!("Total batches: {}", total_patches / PATCHES_PER_BATCH);
        println!("Initial every: {} batches", initial_every);

        // Scenario A: Single initial at start
        let batches_single = create_realistic_batches(&doc_id_single, 1, total_patches, None);
        println!("Single initial batches: {}", batches_single.len());
        for batch in batches_single {
            database::save_batch(batch).await.expect("save failed");
        }

        // Scenario B: Periodic initials
        let batches_periodic =
            create_realistic_batches(&doc_id_periodic, 1, total_patches, Some(initial_every));
        let initial_count = batches_periodic.iter().filter(|b| b.is_initial).count();
        println!(
            "Periodic initial batches: {} ({} initials)",
            batches_periodic.len(),
            initial_count
        );
        for batch in batches_periodic {
            database::save_batch(batch).await.expect("save failed");
        }

        let query_ts = u128::MAX;

        // Benchmark single initial
        let iterations = 3;
        let mut single_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            invalidate_doc_cache(doc_id_single.clone());
            let start = Instant::now();
            let result = get_blocks(doc_id_single.clone(), vec![1], query_ts).await;
            single_total += start.elapsed();
            assert!(result.is_ok(), "Single initial failed: {:?}", result.err());
        }

        // Benchmark periodic
        let mut periodic_total = std::time::Duration::ZERO;
        for _ in 0..iterations {
            invalidate_doc_cache(doc_id_periodic.clone());
            let start = Instant::now();
            let result = get_blocks(doc_id_periodic.clone(), vec![1], query_ts).await;
            periodic_total += start.elapsed();
            assert!(result.is_ok(), "Periodic failed: {:?}", result.err());
        }

        let single_avg = single_total / iterations as u32;
        let periodic_avg = periodic_total / iterations as u32;
        let speedup = single_total.as_nanos() as f64 / periodic_total.as_nanos() as f64;

        println!("Single initial:   {:?}", single_avg);
        println!("Periodic initial: {:?}", periodic_avg);
        println!("Speedup: {:.1}x", speedup);

        assert!(
            speedup > 3.0,
            "Expected speedup from periodic initials, got {:.1}x",
            speedup
        );

        cleanup(&doc_id_single).await;
        cleanup(&doc_id_periodic).await;
    }

    #[tokio::test]
    async fn bench_parallel_blocks() {
        let doc_id = setup_bench().await;
        cleanup(&doc_id).await;

        let num_blocks = 20u64;
        let patches_per_block = 500;

        println!("\n=== Parallel Blocks Benchmark ===");
        println!("Blocks: {}", num_blocks);
        println!("Patches per block: {}", patches_per_block);
        println!(
            "Batches per block: {}",
            patches_per_block / PATCHES_PER_BATCH
        );

        for block_id in 1..=num_blocks {
            let batches = create_realistic_batches(&doc_id, block_id, patches_per_block, None);
            for batch in batches {
                database::save_batch(batch).await.expect("save failed");
            }
        }

        let block_ids: Vec<u64> = (1..=num_blocks).collect();

        invalidate_doc_cache(doc_id.clone());
        let start = Instant::now();
        let result = get_blocks(doc_id.clone(), block_ids.clone(), u128::MAX)
            .await
            .unwrap();
        let parallel_time = start.elapsed();

        assert_eq!(result.blocks.len(), num_blocks as usize);

        invalidate_doc_cache(doc_id.clone());
        let start = Instant::now();
        for block_id in 1..=num_blocks {
            let _ = get_blocks(doc_id.clone(), vec![block_id], u128::MAX)
                .await
                .unwrap();
        }
        let sequential_time = start.elapsed();

        let speedup = sequential_time.as_nanos() as f64 / parallel_time.as_nanos() as f64;

        println!("Parallel:   {:?}", parallel_time);
        println!("Sequential: {:?}", sequential_time);
        println!("Speedup: {:.1}x", speedup);

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn bench_stress_large_document() {
        let doc_id = setup_bench().await;
        cleanup(&doc_id).await;

        let num_blocks: u64 = 100;
        let patches_per_block: usize = 10_000;
        let initial_every: usize = 100;

        println!("\n========================================");
        println!("=== STRESS TEST ===");
        println!("========================================");
        println!("Blocks: {}", num_blocks);
        println!("Patches per block: {}", patches_per_block);
        println!(
            "Batches per block: {}",
            patches_per_block / PATCHES_PER_BATCH
        );
        println!("Total patches: {}", num_blocks as usize * patches_per_block);
        println!("Initial every: {} batches", initial_every);

        // ============================================
        // SCENARIO A: Single initial per block
        // ============================================
        println!("\n--- Scenario A: Single Initial Per Block ---");
        let setup_start = Instant::now();

        for block_id in 1..=num_blocks {
            let batches = create_realistic_batches(&doc_id, block_id, patches_per_block, None);
            for batch in batches {
                database::save_batch(batch).await.expect("save failed");
            }
        }
        println!("Batches saved in {:?}", setup_start.elapsed());

        let block_ids: Vec<u64> = (1..=num_blocks).collect();
        let query_ts = u128::MAX;

        // Cold reconstruction
        invalidate_doc_cache(doc_id.clone());
        let start = Instant::now();
        let result = get_blocks(doc_id.clone(), block_ids.clone(), query_ts)
            .await
            .expect("get_blocks failed");
        let cold_time = start.elapsed();

        println!("\nSingle Initial Results:");
        println!("  Blocks retrieved: {}", result.blocks.len());
        println!("  Cold reconstruction: {:?}", cold_time);
        println!("  Per block (cold): {:?}", cold_time / num_blocks as u32);

        // Wait for snapshots
        println!("\n  Waiting for snapshots...");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let snapshot_count = database::count_snapshots_by_doc(doc_id.clone())
            .await
            .unwrap_or(0);
        println!("  Snapshots created: {}", snapshot_count);

        // With snapshots (cold, but DB has snapshots now)
        invalidate_doc_cache(doc_id.clone());
        let start = Instant::now();
        let _ = get_blocks(doc_id.clone(), block_ids.clone(), query_ts)
            .await
            .expect("get_blocks failed");
        let snapshot_time = start.elapsed();

        println!("  With snapshots: {:?}", snapshot_time);
        println!(
            "  Snapshot speedup: {:.1}x",
            cold_time.as_nanos() as f64 / snapshot_time.as_nanos() as f64
        );

        // Warm (cache hit)
        let start = Instant::now();
        let _ = get_blocks(doc_id.clone(), block_ids.clone(), query_ts)
            .await
            .expect("get_blocks failed");
        let warm_time = start.elapsed();

        println!("  Warm (cached): {:?}", warm_time);
        println!(
            "  Cache speedup vs cold: {:.1}x",
            cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64
        );

        cleanup(&doc_id).await;

        // ============================================
        // SCENARIO B: Periodic initials
        // ============================================
        let doc_id_periodic = format!("{}_periodic", doc_id);
        cleanup(&doc_id_periodic).await;

        println!(
            "\n--- Scenario B: Periodic Initials (every {} batches) ---",
            initial_every
        );
        let setup_start = Instant::now();

        for block_id in 1..=num_blocks {
            let batches = create_realistic_batches(
                &doc_id_periodic,
                block_id,
                patches_per_block,
                Some(initial_every),
            );
            for batch in batches {
                database::save_batch(batch).await.expect("save failed");
            }
        }
        println!("Batches saved in {:?}", setup_start.elapsed());

        // Cold reconstruction
        invalidate_doc_cache(doc_id_periodic.clone());
        let start = Instant::now();
        let result = get_blocks(doc_id_periodic.clone(), block_ids.clone(), query_ts)
            .await
            .expect("get_blocks failed");
        let periodic_cold_time = start.elapsed();

        println!("\nPeriodic Initial Results:");
        println!("  Blocks retrieved: {}", result.blocks.len());
        println!("  Cold reconstruction: {:?}", periodic_cold_time);
        println!(
            "  Per block (cold): {:?}",
            periodic_cold_time / num_blocks as u32
        );

        // Warm
        let start = Instant::now();
        let _ = get_blocks(doc_id_periodic.clone(), block_ids.clone(), query_ts)
            .await
            .expect("get_blocks failed");
        let periodic_warm_time = start.elapsed();

        println!("  Warm (cached): {:?}", periodic_warm_time);
        println!(
            "  Cache speedup: {:.1}x",
            periodic_cold_time.as_nanos() as f64 / periodic_warm_time.as_nanos() as f64
        );

        // ============================================
        // COMPARISON
        // ============================================
        println!("\n========================================");
        println!("=== COMPARISON ===");
        println!("========================================");
        println!(
            "Periodic vs Single Initial (cold): {:.1}x faster",
            cold_time.as_nanos() as f64 / periodic_cold_time.as_nanos() as f64
        );
        println!(
            "Periodic vs Single Initial (warm): {:.1}x faster",
            warm_time.as_nanos() as f64 / periodic_warm_time.as_nanos() as f64
        );

        // ============================================
        // HISTORICAL QUERY TEST
        // ============================================
        println!("\n--- Historical Query Test ---");

        let batches_per_block = patches_per_block / PATCHES_PER_BATCH;
        let test_points = [
            ("10%", (batches_per_block / 10) as u128 * 1000),
            ("50%", (batches_per_block / 2) as u128 * 1000),
            ("90%", (batches_per_block * 9 / 10) as u128 * 1000),
        ];

        invalidate_doc_cache(doc_id_periodic.clone());

        for (name, ts) in test_points {
            let start = Instant::now();
            let result = get_blocks(doc_id_periodic.clone(), block_ids.clone(), ts)
                .await
                .expect("get_blocks failed");
            let query_time = start.elapsed();

            println!(
                "  Query at {} (ts={}): {:?} ({} blocks)",
                name,
                ts,
                query_time,
                result.blocks.len()
            );
        }

        // Incremental test
        println!("\n--- Incremental Query Test ---");
        invalidate_doc_cache(doc_id_periodic.clone());

        let mid_ts = (batches_per_block / 2) as u128 * 1000;
        let start = Instant::now();
        let _ = get_blocks(doc_id_periodic.clone(), block_ids.clone(), mid_ts)
            .await
            .expect("get_blocks failed");
        let first_query = start.elapsed();
        println!("  First query (50%, cold): {:?}", first_query);

        let later_ts = (batches_per_block * 3 / 4) as u128 * 1000;
        let start = Instant::now();
        let _ = get_blocks(doc_id_periodic.clone(), block_ids.clone(), later_ts)
            .await
            .expect("get_blocks failed");
        let incremental_query = start.elapsed();
        println!(
            "  Incremental query (75%, from cache): {:?}",
            incremental_query
        );
        println!(
            "  Incremental speedup: {:.1}x",
            first_query.as_nanos() as f64 / incremental_query.as_nanos() as f64
        );

        cleanup(&doc_id_periodic).await;

        println!("\n========================================");
        println!("=== STRESS TEST COMPLETE ===");
        println!("========================================");
    }
}
