use crate::error::AppError;

/// 自前の Process Tap と、それを含む集約デバイス。捨てると両方を破棄する。
pub struct TapAggregate {
    device_name: String,
}

impl TapAggregate {
    /// 既定の出力デバイスを対象に、全プロセスの Tap（ミュートしない）と、
    /// それを入れた集約デバイス `SnipWave Loopback <pid>` を作る。
    pub fn create_for_default_output() -> Result<Self, AppError> {
        Err(AppError::LoopbackUnsupported("未実装".to_string()))
    }

    /// cpal の `host.devices()` から探すときの名前。
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

impl Drop for TapAggregate {
    fn drop(&mut self) {}
}

/// 前回の残骸（名前が `SnipWave Loopback` で始まる集約デバイス）を破棄し、破棄した数を返す。起動時に 1 回呼ぶ。
#[allow(dead_code)] // T7b で capture.rs から呼ぶ
pub fn destroy_stale_aggregates() -> Result<usize, AppError> {
    Ok(0)
}
