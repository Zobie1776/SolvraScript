use anyhow::Result;
use tracing::warn;

/// Dispatches a compute shader. The current implementation only logs the request but keeps the
/// API surface intact so that consumers can experiment with GPU acceleration.
pub async fn dispatch(shader: &str, workgroup_size: (u32, u32, u32)) -> Result<()> {
    warn!(%shader, ?workgroup_size, "GPU dispatch invoked (no-op)");
    Ok(())
}
