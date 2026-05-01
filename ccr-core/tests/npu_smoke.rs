//! Feature-gated NPU smoke test.
//!
//! Skipped at compile time unless `--features openvino`; skipped at run time
//! unless `OPENVINO_NPU_AVAILABLE=1`. Verifies the raw OpenVINO embedder
//! actually engaged (asserts `ov_embedder_is_active()`), not just that some
//! embedder produced 384-dim L2-normalised vectors.

#![cfg(feature = "openvino")]

use panda_core::summarizer;

fn npu_opted_in() -> bool {
    std::env::var("OPENVINO_NPU_AVAILABLE").ok().as_deref() == Some("1")
}

#[test]
fn npu_smoke_actually_uses_npu() {
    if !npu_opted_in() {
        eprintln!("skipping: OPENVINO_NPU_AVAILABLE != 1");
        return;
    }
    summarizer::set_execution_provider("npu");
    let texts = vec!["error: build failed", "warning: deprecated", "ok"];

    let t0 = std::time::Instant::now();
    let embeddings = summarizer::embed_direct(texts).expect("embed_direct");
    let elapsed = t0.elapsed();

    assert_eq!(embeddings.len(), 3, "expected 3 vectors");
    for (i, v) in embeddings.iter().enumerate() {
        assert_eq!(v.len(), 384, "vec {i} dim mismatch");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-3,
            "vec {i} not L2-normalised: norm={norm}"
        );
    }

    assert!(
        summarizer::ov_embedder_is_active(),
        "raw OV embedder must be active in NPU mode — without this assertion, \
         a silent CPU fallback would let this test pass spuriously"
    );

    eprintln!("npu embed elapsed: {elapsed:?}");
}

#[test]
fn npu_falls_back_to_cpu_when_libopenvino_missing() {
    if !npu_opted_in() {
        eprintln!("skipping: OPENVINO_NPU_AVAILABLE != 1");
        return;
    }
    // Hide libopenvino_c.so so OvEmbedder::try_new fails. ov_lib_path()
    // checks OPENVINO_LIB_PATH first, so pointing it at /dev/null beats
    // any other path on the host.
    std::env::set_var("OPENVINO_LIB_PATH", "/dev/null");
    summarizer::set_execution_provider("npu");

    // Embedding should still succeed via CPU fallback.
    let r = summarizer::embed_direct(vec!["x"]);
    std::env::remove_var("OPENVINO_LIB_PATH");

    assert!(r.is_ok(), "expected CPU fallback to succeed: {:?}", r.err());
    assert!(
        !summarizer::ov_embedder_is_active(),
        "OV embedder must NOT be active when libopenvino is hidden"
    );
}
