//! Bench-only seam: lets `benches/` drive `VirtualTerminal::commit` from
//! outside the crate. Not API.

pub use crate::canvas::Canvas;
pub use crate::render::virtual_terminal::{Content, Desired, VirtualTerminal};

pub fn commit(
    terminal: &mut VirtualTerminal,
    frame: &[Desired],
    content: &mut dyn FnMut() -> Content,
) -> usize {
    terminal.commit_for_bench(frame, content).len()
}
