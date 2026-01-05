use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;
use reqwest::Client;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;

const BASE_URL: &str = "http://localhost:8081";

// Atomic counter for unique user IDs across benchmarks
static USER_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper to create a minimal JWT for testing with a unique user ID
fn create_test_jwt(suffix: &str) -> String {
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
        iss: String,
        plan: String,
    }

    let counter = USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let claims = Claims {
        sub: format!("bench-user-{}-{}", suffix, counter),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iss: "test-issuer".to_string(),
        plan: "pro".to_string(), // Pro plan for higher quota
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("test-secret".as_bytes()),
    )
    .unwrap()
}

/// Helper struct for benchmark document setup
struct BenchDocument {
    document_id: String,
    signing_key: SigningKey,
    public_key_hex: String,
}

impl BenchDocument {
    fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.to_bytes());
        let document_id = format!("bench-doc-{}", uuid::Uuid::new_v4());

        Self {
            document_id,
            signing_key,
            public_key_hex,
        }
    }

    fn sign_upload(&self, hash: &str, timestamp: i64) -> String {
        let message = format!("{}{}{}", hash, self.document_id, timestamp);
        let signature = self.signing_key.sign(message.as_bytes());
        hex::encode(signature.to_bytes())
    }
}

/// Setup a document for benchmarking (returns None on connection failure)
async fn setup_document(client: &Client, jwt: &str) -> Option<BenchDocument> {
    let doc = BenchDocument::new();

    match client
        .post(&format!("{}/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", jwt))
        .json(&serde_json::json!({
            "document_id": doc.document_id,
            "public_key": doc.public_key_hex,
        }))
        .send()
        .await
    {
        Ok(_) => Some(doc),
        Err(_) => None, // Connection failed, skip this iteration
    }
}

/// Benchmark: GET /quota endpoint
fn bench_quota_endpoint(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();
    let jwt = create_test_jwt("quota");

    c.bench_function("quota_endpoint", |b| {
        b.to_async(&rt).iter(|| async {
            let response = client
                .get(&format!("{}/quota", BASE_URL))
                .header("Authorization", format!("Bearer {}", jwt))
                .send()
                .await
                .expect("Failed to send request");

            black_box(response.status());
        });
    });
}

/// Benchmark: Blob upload (various sizes)
/// Uses unique JWT per iteration to avoid rate limiting
fn bench_blob_upload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();

    let sizes = vec![
        ("1KB", 1024usize),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
        ("5MB", 5 * 1024 * 1024),   // At multipart threshold
        ("10MB", 10 * 1024 * 1024), // Above multipart threshold
    ];

    let mut group = c.benchmark_group("blob_upload");
    group.sample_size(10); // Reduce sample size for large uploads

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        let blob_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        group.bench_with_input(BenchmarkId::from_parameter(name), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = client.clone();
                let blob_data = blob_data.clone();

                async move {
                    // Create unique user and document per iteration to avoid rate limits
                    let jwt = create_test_jwt("upload");
                    let Some(doc) = setup_document(&client, &jwt).await else {
                        return; // Skip iteration on connection failure
                    };

                    let hash = blake3::hash(&blob_data).to_hex().to_string();
                    let timestamp = chrono::Utc::now().timestamp();
                    let message = format!("{}{}{}", hash, doc.document_id, timestamp);
                    let signature = doc.signing_key.sign(message.as_bytes());
                    let signature_hex = hex::encode(signature.to_bytes());

                    let url = format!(
                        "{}/blobs?document_id={}&signature={}&timestamp={}",
                        BASE_URL, doc.document_id, signature_hex, timestamp
                    );

                    if let Ok(response) = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", jwt))
                        .body(blob_data)
                        .send()
                        .await
                    {
                        let status = response.status();
                        black_box(status);

                        // Clean up
                        if status.is_success() {
                            let _ = client
                                .delete(&format!("{}/blobs/{}", BASE_URL, hash))
                                .header("Authorization", format!("Bearer {}", jwt))
                                .send()
                                .await;
                        }
                    }
                }
            });
        });
    }

    group.finish();
}

/// Benchmark: Blob download (various sizes)
/// Downloads are public so no rate limiting issues
fn bench_blob_download(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();

    let sizes = vec![
        ("1KB", 1024usize),
        ("10KB", 10 * 1024),
        ("100KB", 100 * 1024),
        ("1MB", 1024 * 1024),
        ("5MB", 5 * 1024 * 1024),
    ];

    let mut group = c.benchmark_group("blob_download");
    group.sample_size(10);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        let blob_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let hash = blake3::hash(&blob_data).to_hex().to_string();

        // Upload the blob first with a unique user
        let jwt = create_test_jwt("download_setup");
        let Some(doc) = rt.block_on(setup_document(&client, &jwt)) else {
            eprintln!(
                "Failed to setup document for download bench, skipping {}",
                name
            );
            continue;
        };
        rt.block_on(async {
            let timestamp = chrono::Utc::now().timestamp();
            let signature = doc.sign_upload(&hash, timestamp);

            let url = format!(
                "{}/blobs?document_id={}&signature={}&timestamp={}",
                BASE_URL, doc.document_id, signature, timestamp
            );

            let _ = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", &jwt))
                .body(blob_data.clone())
                .send()
                .await;
        });

        group.bench_with_input(BenchmarkId::from_parameter(name), &size, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = client.clone();
                let hash = hash.clone();

                async move {
                    let response = client
                        .get(&format!("{}/blobs/{}", BASE_URL, hash))
                        .send()
                        .await
                        .expect("Failed to download blob");

                    let data = response.bytes().await.expect("Failed to read bytes");
                    black_box(data);
                }
            });
        });

        // Clean up the uploaded blob
        rt.block_on(async {
            let _ = client
                .delete(&format!("{}/blobs/{}", BASE_URL, hash))
                .header("Authorization", format!("Bearer {}", &jwt))
                .send()
                .await;
        });
    }

    group.finish();
}

/// Benchmark: Concurrent quota requests
/// Uses unique JWT per concurrent request to avoid rate limiting
fn bench_concurrent_quota_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Reduced max concurrency to avoid overwhelming the server
    let concurrency_levels = vec![1, 5, 10, 20];

    let mut group = c.benchmark_group("concurrent_quota");
    group.sample_size(10);

    for concurrency in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| {
                    async move {
                        let mut handles = vec![];

                        for i in 0..concurrency {
                            // Unique JWT per concurrent request
                            let jwt = create_test_jwt(&format!("concurrent_quota_{}", i));

                            let handle = tokio::spawn(async move {
                                let client = Client::new();
                                // Use ok() to handle connection errors gracefully
                                if let Ok(response) = client
                                    .get(&format!("{}/quota", BASE_URL))
                                    .header("Authorization", format!("Bearer {}", jwt))
                                    .send()
                                    .await
                                {
                                    black_box(response.status());
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Concurrent blob uploads
/// Uses unique user per concurrent upload to avoid rate limiting
fn bench_concurrent_uploads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::builder()
        .pool_max_idle_per_host(100)
        .build()
        .unwrap();

    let concurrency_levels = vec![1, 5, 10, 25];

    let mut group = c.benchmark_group("concurrent_upload");
    group.sample_size(10);

    for concurrency in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| {
                    let client = client.clone();

                    async move {
                        let mut handles = vec![];

                        for i in 0..concurrency {
                            let client = client.clone();

                            let handle = tokio::spawn(async move {
                                // Unique user per concurrent upload
                                let jwt = create_test_jwt(&format!("concurrent_upload_{}", i));
                                let Some(doc) = setup_document(&client, &jwt).await else {
                                    return; // Skip on connection failure
                                };

                                let blob_data = format!("Concurrent upload {}", i).into_bytes();
                                let hash = blake3::hash(&blob_data).to_hex().to_string();
                                let timestamp = chrono::Utc::now().timestamp();
                                let message = format!("{}{}{}", hash, doc.document_id, timestamp);
                                let signature = doc.signing_key.sign(message.as_bytes());
                                let signature_hex = hex::encode(signature.to_bytes());

                                let url = format!(
                                    "{}/blobs?document_id={}&signature={}&timestamp={}",
                                    BASE_URL, doc.document_id, signature_hex, timestamp
                                );

                                if let Ok(response) = client
                                    .post(&url)
                                    .header("Authorization", format!("Bearer {}", jwt))
                                    .body(blob_data)
                                    .send()
                                    .await
                                {
                                    black_box(response.status());

                                    // Cleanup
                                    let _ = client
                                        .delete(&format!("{}/blobs/{}", BASE_URL, hash))
                                        .header("Authorization", format!("Bearer {}", jwt))
                                        .send()
                                        .await;
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Concurrent blob downloads
/// Downloads are public, so no rate limiting by user
fn bench_concurrent_downloads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::builder()
        .pool_max_idle_per_host(100)
        .build()
        .unwrap();

    // Setup document and upload a blob
    let jwt = create_test_jwt("concurrent_download_setup");
    let Some(doc) = rt.block_on(setup_document(&client, &jwt)) else {
        eprintln!("Failed to setup document for concurrent download bench, skipping");
        return;
    };
    let blob_data = vec![0u8; 100 * 1024]; // 100KB blob
    let hash = blake3::hash(&blob_data).to_hex().to_string();

    rt.block_on(async {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = doc.sign_upload(&hash, timestamp);

        let url = format!(
            "{}/blobs?document_id={}&signature={}&timestamp={}",
            BASE_URL, doc.document_id, signature, timestamp
        );

        let _ = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &jwt))
            .body(blob_data)
            .send()
            .await;
    });

    // Reduced max concurrency to avoid overwhelming the server
    let concurrency_levels = vec![1, 10, 20, 50];

    let mut group = c.benchmark_group("concurrent_download");
    group.sample_size(10);

    for concurrency in concurrency_levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| {
                    let client = client.clone();
                    let hash = hash.clone();

                    async move {
                        let mut handles = vec![];

                        for _ in 0..concurrency {
                            let client = client.clone();
                            let hash = hash.clone();

                            let handle = tokio::spawn(async move {
                                // Use ok() to handle connection errors gracefully
                                if let Ok(response) = client
                                    .get(&format!("{}/blobs/{}", BASE_URL, hash))
                                    .send()
                                    .await
                                {
                                    if let Ok(data) = response.bytes().await {
                                        black_box(data);
                                    }
                                }
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }

    // Cleanup
    rt.block_on(async {
        let _ = client
            .delete(&format!("{}/blobs/{}", BASE_URL, hash))
            .header("Authorization", format!("Bearer {}", &jwt))
            .send()
            .await;
    });

    group.finish();
}

/// Benchmark: Rate limiting overhead (sequential requests with unique users)
fn bench_rate_limiting_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();

    c.bench_function("rate_limiting_overhead_10_requests", |b| {
        b.to_async(&rt).iter(|| async {
            for i in 0..10 {
                // Unique user per request to avoid rate limiting
                let jwt = create_test_jwt(&format!("rate_limit_{}", i));
                if let Ok(response) = client
                    .get(&format!("{}/quota", BASE_URL))
                    .header("Authorization", format!("Bearer {}", jwt))
                    .send()
                    .await
                {
                    black_box(response.status());
                }
            }
        });
    });
}

/// Benchmark: JWT validation overhead
fn bench_jwt_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();

    c.bench_function("jwt_validation", |b| {
        b.to_async(&rt).iter(|| async {
            // Create a fresh JWT each time to measure validation (and avoid rate limits)
            let jwt = create_test_jwt("jwt_bench");

            if let Ok(response) = client
                .get(&format!("{}/quota", BASE_URL))
                .header("Authorization", format!("Bearer {}", jwt))
                .send()
                .await
            {
                black_box(response.status());
            }
        });
    });
}

/// Benchmark: Ed25519 signature verification (via upload)
/// Uses unique user per iteration to avoid rate limits
fn bench_signature_verification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = Client::new();
    let blob_data = b"Small blob for signature benchmark".to_vec();

    c.bench_function("signature_verification", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let blob_data = blob_data.clone();

            async move {
                // Unique user per iteration
                let jwt = create_test_jwt("sig_bench");
                let Some(doc) = setup_document(&client, &jwt).await else {
                    return; // Skip on connection failure
                };

                let hash = blake3::hash(&blob_data).to_hex().to_string();
                let timestamp = chrono::Utc::now().timestamp();
                let message = format!("{}{}{}", hash, doc.document_id, timestamp);
                let signature = doc.signing_key.sign(message.as_bytes());
                let signature_hex = hex::encode(signature.to_bytes());

                let url = format!(
                    "{}/blobs?document_id={}&signature={}&timestamp={}",
                    BASE_URL, doc.document_id, signature_hex, timestamp
                );

                if let Ok(response) = client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", jwt))
                    .body(blob_data)
                    .send()
                    .await
                {
                    let status = response.status();
                    black_box(status);

                    // Cleanup
                    if status.is_success() {
                        let _ = client
                            .delete(&format!("{}/blobs/{}", BASE_URL, hash))
                            .header("Authorization", format!("Bearer {}", jwt))
                            .send()
                            .await;
                    }
                }
            }
        });
    });
}

criterion_group!(
    benches,
    bench_quota_endpoint,
    bench_blob_upload,
    bench_blob_download,
    bench_concurrent_quota_requests,
    bench_concurrent_uploads,
    bench_concurrent_downloads,
    bench_rate_limiting_overhead,
    bench_jwt_validation,
    bench_signature_verification,
);
criterion_main!(benches);
