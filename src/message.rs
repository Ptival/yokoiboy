#[derive(Clone, Debug, Hash)]
pub enum Message {
    Pause,
    Quit,
    RunNextInstruction,
    BeginRunUntilBreakpoint,
    ContinueRunUntilBreakpoint,
}
