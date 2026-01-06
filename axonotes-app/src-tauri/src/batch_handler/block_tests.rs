#[cfg(test)]
mod tests {
    use crate::batch_handler::block_getter::*;
    use crate::batch_handler::block_types::{Block, BlockContent, ParagraphV1};
    use crate::database;
    use crate::encryption::batch::{BatchData, DecryptedBatch, Patch};
    use spacetimedb_sdk::Identity;

    // ==========================================
    // Test Helpers
    // ==========================================

    fn make_test_block(id: u64, text: &str) -> Block {
        Block::new(
            id,
            1234567890,
            BlockContent::ParagraphV1(ParagraphV1 {
                deleted: None,
                author: Identity::from_byte_array([0u8; 32]),
                group_id: "main".to_string(),
                group_row: "0".to_string(),
                text: text.to_string(),
                formatting: vec![],
            }),
        )
    }

    fn get_block_text(block: &Block) -> &str {
        match &block.content {
            BlockContent::ParagraphV1(p) => &p.text,
            _ => panic!("Expected ParagraphV1"),
        }
    }

    fn make_initial_patch(block: &Block, time_delta: u8) -> Patch {
        crate::batch_handler::block_type_helpers::encode_initial_patch(0, time_delta, block)
            .expect("Failed to encode initial patch")
    }

    fn make_patch(base: &Block, new: &Block, time_delta: u8) -> Patch {
        crate::batch_handler::block_type_helpers::encode_patch(0, time_delta, base, new)
            .expect("Failed to encode patch")
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

    fn make_snapshot(
        doc_id: &str,
        block_id: u64,
        timestamp: u128,
        patches: Vec<Patch>,
    ) -> DecryptedBatch {
        DecryptedBatch {
            batch_id: format!("snapshot:{}:{}:{}", doc_id, block_id, timestamp),
            doc_id: doc_id.to_string(),
            timestamp,
            is_initial: true,
            batch_data: BatchData { block_id, patches },
        }
    }

    async fn setup_test() -> String {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let temp_dir = std::env::temp_dir().join("axonotes_test_db");
            std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
            let _ = database::init_paths(temp_dir);
        });

        if !database::is_unlocked().await {
            database::unlock_db("".to_string())
                .await
                .expect("Failed to unlock db");
        }

        format!("test_doc_{}", uuid::Uuid::new_v4())
    }

    async fn cleanup(doc_id: &str) {
        let _ = database::delete_batches_by_doc(doc_id.to_string()).await;
        invalidate_doc_cache(doc_id.to_string());
    }

    // ==========================================
    // Basic Functionality Tests
    // ==========================================

    #[tokio::test]
    async fn test_single_block_single_patch() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let block = make_test_block(1, "hello");
        let patch = make_initial_patch(&block, 10);
        let batch = make_batch(&doc_id, 1, 0, 100, true, vec![patch]);

        database::save_batch(batch).await.expect("save failed");

        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");

        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].timestamp, 150);
        assert_eq!(get_block_text(&result.blocks[0].block), "hello");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_multiple_batches() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3 = make_test_block(1, "v3");

        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        let batch2 = make_batch(&doc_id, 1, 1, 200, false, vec![make_patch(&v1, &v2, 2)]);
        let batch3 = make_batch(&doc_id, 1, 2, 300, false, vec![make_patch(&v2, &v3, 2)]);

        database::save_batch(batch1).await.expect("save failed");
        database::save_batch(batch2).await.expect("save failed");
        database::save_batch(batch3).await.expect("save failed");

        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");

        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].timestamp, 310);
        assert_eq!(get_block_text(&result.blocks[0].block), "v3");

        cleanup(&doc_id).await;
    }

    // ==========================================
    // Snapshot Tests (BUG FIX VERIFICATION)
    // ==========================================

    #[tokio::test]
    async fn test_snapshot_used_instead_of_batch() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        // Create initial batch
        let v1 = make_test_block(1, "v1");
        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        database::save_batch(batch1).await.expect("save failed");

        // Create snapshot at ts=200 with different content
        let v2_snapshot = make_test_block(1, "v2_from_snapshot");
        let snapshot = make_snapshot(&doc_id, 1, 200, vec![make_initial_patch(&v2_snapshot, 2)]);
        database::save_snapshot(snapshot)
            .await
            .expect("save snapshot failed");

        // Create batch at same ts=200 (should be ignored in favor of snapshot)
        let v2_batch = make_test_block(1, "v2_from_batch_WRONG");
        let batch2 = make_batch(
            &doc_id,
            1,
            1,
            200,
            true,
            vec![make_initial_patch(&v2_batch, 2)],
        );
        database::save_batch(batch2).await.expect("save failed");

        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");

        // Should get snapshot content, not batch content
        assert_eq!(get_block_text(&result.blocks[0].block), "v2_from_snapshot");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_snapshot_in_range_query() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3 = make_test_block(1, "v3_snapshot");
        let v4 = make_test_block(1, "v4");

        // Initial batch at ts=100
        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        database::save_batch(batch1).await.expect("save failed");

        // Delta batch at ts=200
        let batch2 = make_batch(&doc_id, 1, 1, 200, false, vec![make_patch(&v1, &v2, 2)]);
        database::save_batch(batch2).await.expect("save failed");

        // Snapshot at ts=300 (NOT in batches table!)
        let snapshot = make_snapshot(&doc_id, 1, 300, vec![make_initial_patch(&v3, 2)]);
        database::save_snapshot(snapshot)
            .await
            .expect("save snapshot failed");

        // Delta batch at ts=400 that depends on v3
        let batch3 = make_batch(&doc_id, 1, 2, 400, false, vec![make_patch(&v3, &v4, 2)]);
        database::save_batch(batch3).await.expect("save failed");

        // First, get state at ts=210 to cache v2
        let _ = get_blocks(doc_id.clone(), vec![1], 210)
            .await
            .expect("get_blocks failed");

        // Now get state at ts=500 - this MUST use snapshot at 300, not cached v2!
        let result = get_blocks(doc_id.clone(), vec![1], 500)
            .await
            .expect("get_blocks failed");

        // If snapshots work correctly, we get v4 (built from v3 snapshot)
        // If broken, we'd try to apply v4's delta to v2 and get wrong result or error
        assert_eq!(result.blocks.len(), 1);
        assert_eq!(get_block_text(&result.blocks[0].block), "v4");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_cache_vs_newer_initial() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3_new_initial = make_test_block(1, "v3_new_initial");
        let v4 = make_test_block(1, "v4");

        // Initial at ts=100, patches at 110
        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        database::save_batch(batch1).await.expect("save failed");

        // Delta at ts=200, patches at 210
        let batch2 = make_batch(&doc_id, 1, 1, 200, false, vec![make_patch(&v1, &v2, 2)]);
        database::save_batch(batch2).await.expect("save failed");

        // Populate cache with v2 at ts=210
        let result = get_blocks(doc_id.clone(), vec![1], 210)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v2");

        // Now add a NEW initial at ts=300 (simulating sync from another device)
        let batch3 = make_batch(
            &doc_id,
            1,
            2,
            300,
            true,
            vec![make_initial_patch(&v3_new_initial, 2)],
        );
        database::save_batch(batch3).await.expect("save failed");

        // And a delta at ts=400
        let batch4 = make_batch(
            &doc_id,
            1,
            3,
            400,
            false,
            vec![make_patch(&v3_new_initial, &v4, 2)],
        );
        database::save_batch(batch4).await.expect("save failed");

        // Request at ts=500 - should use initial at 300, not cached v2 at 210
        let result = get_blocks(doc_id.clone(), vec![1], 500)
            .await
            .expect("get_blocks failed");

        assert_eq!(get_block_text(&result.blocks[0].block), "v4");
        assert_eq!(result.blocks[0].timestamp, 410);

        cleanup(&doc_id).await;
    }

    // ==========================================
    // Cache Tests
    // ==========================================

    #[tokio::test]
    async fn test_cache_partial_hit() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3 = make_test_block(1, "v3");

        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        let batch2 = make_batch(&doc_id, 1, 1, 200, false, vec![make_patch(&v1, &v2, 2)]);
        let batch3 = make_batch(&doc_id, 1, 2, 300, false, vec![make_patch(&v2, &v3, 2)]);

        database::save_batch(batch1).await.expect("save failed");
        database::save_batch(batch2).await.expect("save failed");
        database::save_batch(batch3).await.expect("save failed");

        // Get v2, caches 110 and 210
        let _ = get_blocks(doc_id.clone(), vec![1], 210)
            .await
            .expect("get_blocks failed");

        // Now get v3 - should use cached 210 as starting point
        let result = get_blocks(doc_id.clone(), vec![1], 310)
            .await
            .expect("get_blocks failed");

        assert_eq!(get_block_text(&result.blocks[0].block), "v3");

        cleanup(&doc_id).await;
    }

    // ==========================================
    // Timestamp Filtering Tests
    // ==========================================

    #[tokio::test]
    async fn test_timestamp_returns_correct_version() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3 = make_test_block(1, "v3");

        let patches = vec![
            make_initial_patch(&v1, 20),
            make_patch(&v1, &v2, 20),
            make_patch(&v2, &v3, 20),
        ];
        let batch = make_batch(&doc_id, 1, 0, 100, true, patches);
        database::save_batch(batch).await.expect("save failed");

        // v1 at 200, v2 at 300, v3 at 400
        let result = get_blocks(doc_id.clone(), vec![1], 200)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v1");

        invalidate_doc_cache(doc_id.clone());

        let result = get_blocks(doc_id.clone(), vec![1], 300)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v2");

        invalidate_doc_cache(doc_id.clone());

        let result = get_blocks(doc_id.clone(), vec![1], 400)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v3");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_timestamp_before_first_patch_returns_empty() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let batch = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 20)]);
        database::save_batch(batch).await.expect("save failed");

        let result = get_blocks(doc_id.clone(), vec![1], 150)
            .await
            .expect("get_blocks failed");
        assert!(result.blocks.is_empty());

        cleanup(&doc_id).await;
    }

    // ==========================================
    // Edge Cases
    // ==========================================

    #[tokio::test]
    async fn test_nonexistent_block() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let result = get_blocks(doc_id.clone(), vec![999], u128::MAX)
            .await
            .expect("get_blocks failed");

        assert!(result.blocks.is_empty());

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_empty_block_list() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let result = get_blocks(doc_id.clone(), vec![], u128::MAX)
            .await
            .expect("get_blocks failed");

        assert!(result.blocks.is_empty());

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_multiple_blocks_parallel() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        for block_id in 1u64..=5 {
            let block = make_test_block(block_id, &format!("block_{}", block_id));
            let batch = make_batch(
                &doc_id,
                block_id,
                0,
                100,
                true,
                vec![make_initial_patch(&block, 2)],
            );
            database::save_batch(batch).await.expect("save failed");
        }

        let result = get_blocks(doc_id.clone(), vec![1, 2, 3, 4, 5], u128::MAX)
            .await
            .expect("get_blocks failed");

        assert_eq!(result.blocks.len(), 5);

        for block_data in &result.blocks {
            let expected_text = format!("block_{}", block_data.block_id);
            assert_eq!(get_block_text(&block_data.block), expected_text);
        }

        cleanup(&doc_id).await;
    }

    // ==========================================
    // Optimization Tests (Fast Path Verification)
    // ==========================================

    #[tokio::test]
    async fn test_known_latest_fast_path() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");

        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        let batch2 = make_batch(&doc_id, 1, 1, 200, false, vec![make_patch(&v1, &v2, 2)]);

        database::save_batch(batch1).await.expect("save failed");
        database::save_batch(batch2).await.expect("save failed");

        // First call - populates cache and sets KNOWN_LATEST
        let result1 = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result1.blocks[0].block), "v2");
        let ts1 = result1.blocks[0].timestamp;
        let seq1 = result1.blocks[0].seq;

        // Second call - should hit KNOWN_LATEST fast path (no DB query)
        let result2 = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result2.blocks[0].block), "v2");
        assert_eq!(result2.blocks[0].timestamp, ts1);
        assert_eq!(result2.blocks[0].seq, seq1);

        // Third call with timestamp beyond latest - should also use fast path
        let result3 = get_blocks(doc_id.clone(), vec![1], u128::MAX - 1)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result3.blocks[0].block), "v2");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_seq_consecutive_detection() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3 = make_test_block(1, "v3");

        // Three patches in one batch: v1 at 110, v2 at 120, v3 at 130
        let patches = vec![
            make_initial_patch(&v1, 2),
            make_patch(&v1, &v2, 2),
            make_patch(&v2, &v3, 2),
        ];
        let batch = make_batch(&doc_id, 1, 0, 100, true, patches);
        database::save_batch(batch).await.expect("save failed");

        // Get all states to populate cache with seq 1, 2, 3
        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v3");
        assert_eq!(result.blocks[0].seq, 3);

        // Now query timestamp 115 (between v1 at 110 and v2 at 120)
        // Cache has seq=1 at ts=110, seq=2 at ts=120
        // Since seq 2 == seq 1 + 1, they're consecutive, no patches between
        // Should return v1 without DB query
        let result = get_blocks(doc_id.clone(), vec![1], 115)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v1");
        assert_eq!(result.blocks[0].timestamp, 110);

        // Query timestamp 125 (between v2 at 120 and v3 at 130)
        // Should return v2 without DB query
        let result = get_blocks(doc_id.clone(), vec![1], 125)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v2");
        assert_eq!(result.blocks[0].timestamp, 120);

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_seq_non_consecutive_requires_db() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let v2 = make_test_block(1, "v2");
        let v3 = make_test_block(1, "v3");
        let v4 = make_test_block(1, "v4");

        // Batch 1: v1 at 110, v2 at 120
        let batch1 = make_batch(
            &doc_id,
            1,
            0,
            100,
            true,
            vec![make_initial_patch(&v1, 2), make_patch(&v1, &v2, 2)],
        );
        database::save_batch(batch1).await.expect("save failed");

        // Batch 2: v3 at 310, v4 at 320
        let batch2 = make_batch(
            &doc_id,
            1,
            1,
            300,
            false,
            vec![make_patch(&v2, &v3, 2), make_patch(&v3, &v4, 2)],
        );
        database::save_batch(batch2).await.expect("save failed");

        // Get v2 (caches seq 1 and 2)
        let result = get_blocks(doc_id.clone(), vec![1], 120)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v2");
        assert_eq!(result.blocks[0].seq, 2);

        // Clear cache except through invalidation to simulate partial cache
        invalidate_doc_cache(doc_id.clone());

        // Get v4 (caches seq 3 and 4)
        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v4");
        assert_eq!(result.blocks[0].seq, 4);

        // Now if we query ts=250 (between v2 at 120 and v3 at 310)
        // Cache might have seq=2 at ts=120 and seq=3 at ts=310
        // But seq 3 != seq 2 + 1 would mean... wait, they ARE consecutive!
        // Actually in this case we need to verify the correct behavior
        let result = get_blocks(doc_id.clone(), vec![1], 250)
            .await
            .expect("get_blocks failed");
        // Should return v2 since that's the last state before ts=250
        assert_eq!(get_block_text(&result.blocks[0].block), "v2");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_cache_invalidation_clears_known_latest() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        let v1 = make_test_block(1, "v1");
        let batch1 = make_batch(&doc_id, 1, 0, 100, true, vec![make_initial_patch(&v1, 2)]);
        database::save_batch(batch1).await.expect("save failed");

        // Populate cache and KNOWN_LATEST
        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v1");

        // Add new batch (this should invalidate cache)
        let v2 = make_test_block(1, "v2");
        let batch2 = make_batch(&doc_id, 1, 1, 200, false, vec![make_patch(&v1, &v2, 2)]);
        database::save_batch(batch2).await.expect("save failed");
        // Note: save_batch calls invalidate_block_cache internally

        // Now get latest - should see v2, not cached v1
        let result = get_blocks(doc_id.clone(), vec![1], u128::MAX)
            .await
            .expect("get_blocks failed");
        assert_eq!(get_block_text(&result.blocks[0].block), "v2");

        cleanup(&doc_id).await;
    }

    #[tokio::test]
    async fn test_parallel_reconstruction_preserves_all_final_states() {
        let doc_id = setup_test().await;
        cleanup(&doc_id).await;

        // Create 10 blocks, each with multiple patches
        for block_id in 1u64..=10 {
            let v1 = make_test_block(block_id, &format!("block_{}_v1", block_id));
            let v2 = make_test_block(block_id, &format!("block_{}_v2", block_id));
            let v3 = make_test_block(block_id, &format!("block_{}_final", block_id));

            let patches = vec![
                make_initial_patch(&v1, 2),
                make_patch(&v1, &v2, 2),
                make_patch(&v2, &v3, 2),
            ];
            let batch = make_batch(&doc_id, block_id, 0, 100, true, patches);
            database::save_batch(batch).await.expect("save failed");
        }

        // First call - parallel reconstruction of all 10 blocks
        let block_ids: Vec<u64> = (1..=10).collect();
        let result1 = get_blocks(doc_id.clone(), block_ids.clone(), u128::MAX)
            .await
            .expect("get_blocks failed");

        assert_eq!(result1.blocks.len(), 10);
        for block_data in &result1.blocks {
            let expected = format!("block_{}_final", block_data.block_id);
            assert_eq!(get_block_text(&block_data.block), expected);
        }

        // Second call - should be fast (all final states cached and fresh)
        let result2 = get_blocks(doc_id.clone(), block_ids.clone(), u128::MAX)
            .await
            .expect("get_blocks failed");

        assert_eq!(result2.blocks.len(), 10);
        for block_data in &result2.blocks {
            let expected = format!("block_{}_final", block_data.block_id);
            assert_eq!(get_block_text(&block_data.block), expected);
        }

        // Verify timestamps and seqs match between calls
        for i in 0..10 {
            assert_eq!(result1.blocks[i].timestamp, result2.blocks[i].timestamp);
            assert_eq!(result1.blocks[i].seq, result2.blocks[i].seq);
        }

        cleanup(&doc_id).await;
    }
}
