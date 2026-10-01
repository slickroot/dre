mod history;
mod state;
mod view;

use std::io::{self, Stdout, Write};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

use crate::key_source::{KeySource, TtyKeySource};
use crate::kitty;
use crate::render::{GlyphCache, Renderer, TerminalRenderer, CACHE_LIMIT};
use crate::tty;
use state::{FlexEffect, FlexState};

#[cfg_attr(test, mockall::automock)]
pub(crate) trait FlexScreen {
    fn render(&mut self, state: &FlexState) -> io::Result<()>;
    fn resize(&mut self) -> io::Result<()>;
}

pub(crate) struct TerminalFlexScreen {
    pub(crate) renderer: TerminalRenderer,
    pub(crate) out: Stdout,
}

impl FlexScreen for TerminalFlexScreen {
    fn render(&mut self, state: &FlexState) -> io::Result<()> {
        self.renderer
            .render(&view::scene(state, self.renderer.area()), &mut self.out)?;
        self.out.flush()
    }

    fn resize(&mut self) -> io::Result<()> {
        self.renderer.on_resize(tty::probe()?);
        Ok(())
    }
}

pub(crate) fn run_loop(keys: &mut dyn KeySource, screen: &mut dyn FlexScreen) -> io::Result<()> {
    let mut state = FlexState::default();
    loop {
        screen.render(&state)?;
        let key = keys.next_key()?;
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
    let renderer = TerminalRenderer::new(window, glyph_source, CACHE_LIMIT);
    let _raw = tty::RawMode::enter(fd)?;
    let resize_fd = tty::install_resize_pipe()?;
    run_loop(
        &mut TtyKeySource { fd, resize_fd },
        &mut TerminalFlexScreen {
            renderer,
            out: stdout,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_source::MockKeySource;
    use mockall::Sequence;

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
                .returning(|_| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move || Ok(key.to_string()));
        }

        run_loop(&mut keys, &mut screen).unwrap();
    }

    #[test]
    fn renders_the_text_as_it_is_typed() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        for (key, shown) in [("i", ""), ("H", ""), ("i", "H"), ("\x03", "Hi")] {
            screen
                .expect_render()
                .withf(move |state| {
                    state.texts_of(&[state.outer_boxes().count() - 1]).concat() == shown
                })
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move || Ok(key.to_string()));
        }

        run_loop(&mut keys, &mut screen).unwrap();
    }

    #[test]
    fn resizes_on_tty_resize_without_reducing() {
        let mut keys = MockKeySource::new();
        let mut screen = MockFlexScreen::new();
        let mut seq = Sequence::new();
        screen
            .expect_render()
            .withf(|state| {
                state
                    .texts_of(&[state.outer_boxes().count() - 1])
                    .is_empty()
            })
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|| Ok("i".to_string()));
        screen
            .expect_render()
            .withf(|state| state.texts_of(&[state.outer_boxes().count() - 1]) == [""])
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|| Ok("a".to_string()));
        screen
            .expect_render()
            .withf(|state| state.texts_of(&[state.outer_boxes().count() - 1])[0] == "a")
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|| Ok(tty::RESIZE.to_string()));
        screen
            .expect_resize()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(()));
        screen
            .expect_render()
            .withf(|state| state.texts_of(&[state.outer_boxes().count() - 1])[0] == "a")
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        keys.expect_next_key()
            .times(1)
            .in_sequence(&mut seq)
            .return_once(|| Ok("\x03".to_string()));

        run_loop(&mut keys, &mut screen).unwrap();
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
                .returning(|_| Ok(()));
            keys.expect_next_key()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move || Ok(key.to_string()));
        }

        run_loop(&mut keys, &mut screen).unwrap();
    }
}
