use boolenum::BoolEnum;

#[derive(BoolEnum, Clone, Debug, Hash)]
pub enum StepBeforeCheckingBreakpoint {
    Yes,
    No,
}

#[derive(Clone, Debug)]
pub enum Message {
    BeginRunUntilBreakpoint(StepBeforeCheckingBreakpoint),
    ContinueRunUntilBreakpoint,
    MouseOnLCDPixel(u8, u8),
    MouseOnTilePalette(u16),
    Pause,
    Quit,
    RunNextInstruction,
}
