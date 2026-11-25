#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppState {
    Idle,
    LiveRecording,
    Editing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEvent {
    Catch,
    RecStart,
    RecStop,
    LiveAutoStopped,
    CloseClip,
    CaptureYieldedNothing,
}

/// 遷移できないときは None（呼び出し側は状態表示に理由を出す）。副作用は持たない。
pub fn next_state(_state: AppState, _event: AppEvent) -> Option<AppState> {
    None
}
