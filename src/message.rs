use boolenum::BoolEnum;

#[derive(BoolEnum, Clone, Debug, Hash)]
pub enum StepBeforeCheckingBreakpoint {
    Yes,
    No,
}

#[derive(Clone, Debug, Hash)]
pub enum Message {
    Pause,
    Quit,
    RunNextInstruction,
    BeginRunUntilBreakpoint(StepBeforeCheckingBreakpoint),
    ContinueRunUntilBreakpoint,
}
