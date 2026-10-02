mod history;
mod state;
mod view;

use std::collections::HashSet;
use std::io::{self, Stdout, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::key_source::{KeySource, TtyKeySource};
use crate::kitty;
use crate::render::{GlyphCache, Renderer, TerminalRenderer, CACHE_LIMIT};
use crate::tty;
use state::{FlexEffect, FlexMode, FlexState};

pub(crate) const BLINK_HALF_MS: u32 = 500;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait FlexScreen {
    fn render(&mut self, state: &FlexState, lit: bool) -> io::Result<()>;
    fn blink(&mut self, lit: bool) -> io::Result<()>;
    fn resize(&mut self) -> io::Result<()>;
}

pub(crate) struct TerminalFlexScreen {
    pub(crate) renderer: TerminalRenderer,
    pub(crate) out: Stdout,
    pub(crate) drawn: Option<HashSet<Vec<usize>>>,
}

impl FlexScreen for TerminalFlexScreen {
    fn render(&mut self, state: &FlexState, lit: bool) -> io::Result<()> {
        let new = new_boxes(&mut self.drawn, state);
        self.renderer.render(
            &view::scene(state, self.renderer.area(), &new, lit),
            lit,
            &mut self.out,
        )?;
        self.out.flush()
    }

    fn blink(&mut self, lit: bool) -> io::Result<()> {
        self.renderer.blink(lit, &mut self.out)?;
        self.out.flush()
    }

    fn resize(&mut self) -> io::Result<()> {
        self.renderer.on_resize(tty::probe()?);
        Ok(())
    }
}

fn new_boxes(drawn: &mut Option<HashSet<Vec<usize>>>, state: &FlexState) -> HashSet<Vec<usize>> {
    let paths: HashSet<Vec<usize>> = state
        .boxes
        .walk()
        .filter(|(_, flex_box)| flex_box.border)
        .map(|(path, _)| path)
        .collect();
    let new = match drawn {
        Some(drawn) => paths.difference(drawn).cloned().collect(),
        None => HashSet::new(),
    };
    *drawn = Some(paths);
    new
}

pub(crate) fn run_loop(keys: &dyn KeySource, screen: &mut dyn FlexScreen) -> io::Result<()> {
    let mut state = FlexState::default();
    let mut lit = true;
    loop {
        // Outside typing mode there is no caret, so the phase carries no meaning
        // and must not survive into the next write session.
        if state.mode != FlexMode::Write {
            lit = true;
        }
        screen.render(&state, lit)?;
        // A tick moves one placement, so it waits for the next key here instead
        // of coming back around to the full render above.
        let key = loop {
            let timeout = (state.mode == FlexMode::Write).then_some(BLINK_HALF_MS);
            let key = keys.next_key(timeout)?;
            if key != tty::TICK {
                break key;
            }
            lit = !lit;
            screen.blink(lit)?;
        };
        if key == tty::RESIZE {
            screen.resize()?;
            continue;
        }
        let effect;
        (state, effect) = state::reduce(state, &key);
        if effect == Some(FlexEffect::Quit) {
            return Ok(());
        }
    }
}

pub fn run() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> io::Result<()> {
    let mut stdout = io::stdout();
    let fd = io::stdin().as_raw_fd();
    kitty::require(&mut stdout, fd)?;
    let window = tty::probe()?;
    let glyph_source = Box::new(GlyphCache::new(window.cell_width, window.cell_height));
    let mut renderer = TerminalRenderer::new(window, glyph_source, CACHE_LIMIT);
    renderer.wezterm = std::env::var("TERM_PROGRAM").is_ok_and(|program| program == "WezTerm");
    let _raw = tty::RawMode::enter(fd)?;
    let resize_fd = tty::install_resize_pipe()?;
    run_loop(
        &TtyKeySource { fd, resize_fd },
        &mut TerminalFlexScreen {
            renderer,
            out: stdout,
            drawn: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_source::MockKeySource;
    use mockall::Sequence;

    fn after(state: FlexState, key: &str) -> FlexState {
        state::reduce(state, key).0
    }

    #[test]
    fn the_first_render_marks_no_box_as_new() {
        let mut drawn = None;

        assert!(new_boxes(&mut drawn, &FlexState::default()).is_empty());
    }

    #[test]
    fn a_marks_the_added_box_as_new_once() {
        let mut drawn = None;
        let state = FlexState::default();
        new_boxes(&mut drawn, &state);

        let state = after(state, "a");

        assert_eq!(
            new_boxes(&mut drawn, &state),
            HashSet::from([state.selected.clone()])
        );
        assert!(new_boxes(&mut drawn, &state).is_empty());
    }

    #[test]
    fn capital_a_marks_the_added_inner_box_as_new() {
        let mut drawn = None;
        let state = FlexState::default();
        new_boxes(&mut drawn, &state);

        let state = after(state, "A");

        let inner = state.boxes.children(&state.selected).pop().unwrap();
        assert_eq!(new_boxes(&mut drawn, &state), HashSet::from([inner]));
    }

    #[test]
    fn a_box_added_again_after_undo_is_new_again() {
        let mut drawn = None;
        let state = FlexState::default();
        new_boxes(&mut drawn, &state);
        let state = after(state, "a");
        new_boxes(&mut drawn, &state);
        let state = after(state, "u");
        new_boxes(&mut drawn, &state);

        let state = after(state, "a");

        assert_eq!(
            new_boxes(&mut drawn, &state),
            HashSet::from([state.selected.clone()])
        );
    }

    #[test]
    fn renders_before_each_key_and_stops_after_ctrl_c() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        for key in ["\x1b[A", "\x03"] {
            screen
                .expect_render()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(key.to_string()));
        }

        run_loop(&keys, &mut screen).unwrap();
    }

    fn last_text(state: &FlexState) -> Option<&str> {
        state
            .outer_boxes()
            .last()
            .and_then(|flex_box| flex_box.text.as_deref())
    }

    #[test]
    fn renders_the_text_as_it_is_typed() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        for (key, shown) in [("i", ""), ("H", ""), ("i", "H"), ("\x03", "Hi")] {
            screen
                .expect_render()
                .withf(move |state, _| last_text(state).unwrap_or_default() == shown)
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(key.to_string()));
        }

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn resizes_on_tty_resize_without_reducing() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        screen
            .expect_render()
            .withf(|state, _| last_text(state).is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("i".to_string()));
        screen
            .expect_render()
            .withf(|state, _| last_text(state) == Some(""))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("a".to_string()));
        screen
            .expect_render()
            .withf(|state, _| last_text(state) == Some("a"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok(tty::RESIZE.to_string()));
        screen
            .expect_resize()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(()));
        screen
            .expect_render()
            .withf(|state, _| last_text(state) == Some("a"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\x03".to_string()));

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn renders_before_each_key_and_stops_after_q_in_move_mode() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        for key in ["\r", "q"] {
            screen
                .expect_render()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(key.to_string()));
        }

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn entering_write_mode_renders_with_the_caret_lit() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        screen
            .expect_render()
            .withf(|_, lit| *lit)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("i".to_string()));
        screen
            .expect_render()
            .withf(|state, lit| *lit && last_text(state) == Some(""))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\x03".to_string()));

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn re_entering_write_mode_on_a_dark_phase_renders_with_the_caret_lit() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        for key in ["i", tty::TICK] {
            screen
                .expect_render()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(key.to_string()));
        }
        screen
            .expect_blink()
            .withf(|lit| !*lit)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\r".to_string()));
        screen
            .expect_render()
            .withf(|state, lit| *lit && state.mode == FlexMode::Move)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("i".to_string()));
        screen
            .expect_render()
            .withf(|state, lit| *lit && state.mode == FlexMode::Write)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\x03".to_string()));

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn a_tick_flips_the_phase_without_re_rendering() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        for key in ["i", tty::TICK] {
            screen
                .expect_render()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(key.to_string()));
        }
        screen
            .expect_blink()
            .withf(|lit| !*lit)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\x03".to_string()));

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn a_tick_is_not_reduced_as_a_keystroke() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        screen
            .expect_render()
            .withf(|state, _| last_text(state).is_none())
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("i".to_string()));
        screen
            .expect_render()
            .withf(|state, _| last_text(state) == Some(""))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("H".to_string()));
        screen
            .expect_render()
            .withf(|state, _| last_text(state) == Some("H"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok(tty::TICK.to_string()));
        screen
            .expect_blink()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("o".to_string()));
        screen
            .expect_render()
            .withf(|state, _| last_text(state) == Some("Ho"))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\x03".to_string()));

        run_loop(&keys, &mut screen).unwrap();
    }

    #[test]
    fn a_timeout_is_asked_for_in_write_mode_only() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        screen
            .expect_render()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .withf(|timeout| timeout.is_none())
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("i".to_string()));
        screen
            .expect_render()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(()));
        keys.expect_next_key()
            .withf(|timeout| *timeout == Some(BLINK_HALF_MS))
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|_| Ok("\x03".to_string()));

        run_loop(&keys, &mut screen).unwrap();
    }
}
