/// T063: Performance benchmarks for protocol adapters
///
/// Benchmarks to verify adapter performance meets requirements:
/// - PassThroughAdapter: < 50μs overhead
/// - Translation adapters: < 1ms P99 latency
/// - Version detection: < 100ms
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mcp_rust_proxy::protocol::{create_adapter, ProtocolVersion};
use serde_json::json;

fn bench_pass_through_adapter(c: &mut Criterion) {
    let mut group = c.benchmark_group("PassThroughAdapter");

    let adapter = create_adapter(ProtocolVersion::V20250618, ProtocolVersion::V20250618);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "test-tool", "arguments": {}}
    });

    group.bench_function("translate_request", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let result = adapter.translate_request(black_box(request.clone())).await;
            black_box(result)
        });
    });

    let response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"content": [{"type": "text", "text": "Hello"}]}
    });

    group.bench_function("translate_response", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let result = adapter
                .translate_response(black_box(response.clone()))
                .await;
            black_box(result)
        });
    });

    group.finish();
}

fn bench_translation_adapters(c: &mut Criterion) {
    let mut group = c.benchmark_group("TranslationAdapters");

    // Test all 6 translation adapter pairs
    let pairs = vec![
        (
            ProtocolVersion::V20241105,
            ProtocolVersion::V20250618,
            "v20241105_to_v20250618",
        ),
        (
            ProtocolVersion::V20250618,
            ProtocolVersion::V20241105,
            "v20250618_to_v20241105",
        ),
        (
            ProtocolVersion::V20241105,
            ProtocolVersion::V20250326,
            "v20241105_to_v20250326",
        ),
        (
            ProtocolVersion::V20250326,
            ProtocolVersion::V20241105,
            "v20250326_to_v20241105",
        ),
        (
            ProtocolVersion::V20250326,
            ProtocolVersion::V20250618,
            "v20250326_to_v20250618",
        ),
        (
            ProtocolVersion::V20250618,
            ProtocolVersion::V20250326,
            "v20250618_to_v20250326",
        ),
    ];

    for (source, target, name) in pairs {
        let adapter = create_adapter(source, target);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "test-tool", "arguments": {"input": "test"}}
        });

        group.bench_with_input(
            BenchmarkId::new("translate_request", name),
            &request,
            |b, req| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let adapter = create_adapter(source, target);
                b.to_async(&rt).iter(|| async {
                    let result = adapter.translate_request(black_box(req.clone())).await;
                    black_box(result)
                });
            },
        );

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [{
                    "name": "tool1",
                    "description": "Test tool",
                    "inputSchema": {"type": "object"}
                }]
            }
        });

        group.bench_with_input(
            BenchmarkId::new("translate_response", name),
            &response,
            |b, resp| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let adapter = create_adapter(source, target);
                b.to_async(&rt).iter(|| async {
                    let result = adapter.translate_response(black_box(resp.clone())).await;
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_version_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("VersionDetection");

    let versions = vec!["2024-11-05", "2025-03-26", "2025-06-18"];

    for version_str in versions {
        group.bench_with_input(
            BenchmarkId::new("from_string", version_str),
            version_str,
            |b, v| {
                b.iter(|| {
                    let (version, _) = ProtocolVersion::from_string(black_box(v));
                    black_box(version)
                });
            },
        );
    }

    group.finish();
}

fn bench_adapter_factory(c: &mut Criterion) {
    let mut group = c.benchmark_group("AdapterFactory");

    let version_pairs = vec![
        (ProtocolVersion::V20241105, ProtocolVersion::V20250618),
        (ProtocolVersion::V20250618, ProtocolVersion::V20241105),
        (ProtocolVersion::V20250326, ProtocolVersion::V20250618),
        (ProtocolVersion::V20250618, ProtocolVersion::V20250326),
    ];

    for (source, target) in version_pairs {
        group.bench_function(
            format!("create_adapter_{}_{}", source.as_str(), target.as_str()),
            |b| {
                b.iter(|| {
                    let adapter = create_adapter(black_box(source), black_box(target));
                    black_box(adapter)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_pass_through_adapter,
    bench_translation_adapters,
    bench_version_detection,
    bench_adapter_factory
);
criterion_main!(benches);
