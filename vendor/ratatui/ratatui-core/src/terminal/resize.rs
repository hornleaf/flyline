use crate::backend::{Backend, ClearType};
use crate::layout::Rect;
use crate::terminal::{Terminal, Viewport};

impl<B: Backend> Terminal<B> {
    /// Updates the Terminal so that internal buffers match the requested area.
    ///
    /// This updates the buffer size used for rendering and triggers a full clear so the next
    /// [`Terminal::draw`] / [`Terminal::try_draw`] paints into a consistent area.
    ///
    /// When the viewport is [`Viewport::Inline`], the `area` argument is treated as the new
    /// terminal size and the viewport origin is recomputed relative to the current cursor position.
    /// Ratatui attempts to keep the cursor at the same relative row within the viewport across
    /// resizes.
    ///
    /// See also: [`Terminal::autoresize`] (automatic resizing during [`Terminal::draw`] /
    /// [`Terminal::try_draw`]).
    ///
    /// For [`Viewport::Fixed`] and [`Viewport::Fullscreen`], `area` becomes the new viewport area.
    /// For [`Viewport::Inline`], `area` is interpreted as the backend's new terminal size and the
    /// viewport origin may move to preserve the cursor's relative row within the inline UI.
    pub fn resize(&mut self, area: Rect) -> Result<(), B::Error> {
        if matches!(self.viewport, Viewport::Inline(_)) {
            let old_width = self.viewport_area.width;
            let new_width = area.width;
            let old_height = self.last_known_area.height;
            let new_height = area.height;

            if (old_width > 0 && old_width != new_width)
                || (old_height > 0 && old_height != new_height)
            {
                self.viewport_top = None;
            }
            let height = match self.viewport {
                Viewport::Inline(h) => {
                    // A zero configured height means "use whatever height the
                    // viewport currently has" (flyline sets the height after
                    // the first render).  Without this, autoresize would keep
                    // collapsing the viewport back to zero rows on every frame
                    // when the backend reports a size that differs from the
                    // stored `last_known_area`.
                    if h == 0 {
                        self.viewport_area.height
                    } else {
                        area.height.min(h)
                    }
                }
                _ => unreachable!(),
            };
            let old_width = self.viewport_area.width;
            let new_width = area.width;

            if old_width > 0 && new_width > 0 && old_width != new_width {
                let prev_buf = &self.buffers[1 - self.current];
                let max_row = self.inline_cursor_y.min(self.viewport_area.height);
                let mut phys_y_from_top = 0u16;

                for i in 0..max_row {
                    let mut last_col = old_width.saturating_sub(1);
                    while last_col > 0 {
                        let cell = &prev_buf[(last_col, i)];
                        let is_whitespace = (cell.symbol() == " " || cell.symbol().is_empty())
                            && cell.style() == crate::style::Style::default();
                        if !is_whitespace {
                            break;
                        }
                        last_col -= 1;
                    }
                    let w_i = if last_col == 0 {
                        let cell = &prev_buf[(0, i)];
                        let is_whitespace = (cell.symbol() == " " || cell.symbol().is_empty())
                            && cell.style() == crate::style::Style::default();
                        if is_whitespace {
                            0
                        } else {
                            (unicode_width::UnicodeWidthStr::width(cell.symbol()) as u16).max(1)
                        }
                    } else {
                        let cell = &prev_buf[(last_col, i)];
                        let symbol_w =
                            (unicode_width::UnicodeWidthStr::width(cell.symbol()) as u16).max(1);
                        last_col + symbol_w
                    };
                    let phys_rows_i = if w_i == 0 {
                        1
                    } else {
                        (w_i + new_width - 1) / new_width
                    };
                    phys_y_from_top += phys_rows_i;
                }
                let cursor_wrap = self.inline_cursor_x / new_width;
                phys_y_from_top += cursor_wrap;

                if phys_y_from_top > 0 {
                    self.backend
                        .move_cursor_relative(0, -(phys_y_from_top as i16))?;
                }
                self.backend.move_cursor_relative(-(old_width as i16), 0)?;
                self.backend.clear_region(ClearType::AfterCursor)?;
                self.inline_cursor_y = 0;
                self.inline_cursor_x = 0;
            }

            self.set_viewport_area(Rect {
                x: 0,
                y: 0,
                width: new_width,
                height,
            });
            self.buffers[0].reset();
            self.buffers[1].reset();
            self.force_full_redraw = true;
            self.last_known_area = area;
            return Ok(());
        }

        let next_area = area;
        self.set_viewport_area(next_area);
        self.clear_viewport()?;
        self.last_known_area = area;
        Ok(())
    }

    /// Queries the backend for size and resizes if it doesn't match the previous size.
    ///
    /// This is called automatically during [`Terminal::draw`] / [`Terminal::try_draw`] for
    /// fullscreen and inline viewports. Fixed viewports are not automatically resized.
    ///
    /// If the size changed, this calls [`Terminal::resize`] and therefore clears the affected
    /// region before the next frame is rendered.
    pub fn autoresize(&mut self) -> Result<(), B::Error> {
        // fixed viewports do not get autoresized
        if matches!(self.viewport, Viewport::Fullscreen | Viewport::Inline(_)) {
            let area: Rect = self.size()?.into();
            // A zero-sized report is not a real terminal size (it can happen
            // during ConPTY/terminal initialisation); ignore it so the stored
            // size does not get poisoned and cause a resize on every frame.
            if area.width == 0 || area.height == 0 {
                return Ok(());
            }
            if area != self.last_known_area {
                self.resize(area)?;
            }
        }
        Ok(())
    }

    /// Resize internal buffers and update the current viewport area.
    ///
    /// This is an internal helper used by [`Terminal::with_options`] and [`Terminal::resize`].
    pub(crate) fn set_viewport_area(&mut self, area: Rect) {
        self.buffers[self.current].resize(area);
        self.buffers[1 - self.current].resize(area);
        self.viewport_area = area;
        self.update_viewport_top_for_height();
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::{Backend, TestBackend};
    use crate::buffer::Buffer;
    use crate::layout::{Position, Rect};
    use crate::terminal::{Terminal, TerminalOptions, Viewport};

    #[test]
    fn resize_fullscreen_updates_viewport_and_buffer_areas() {
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.backend_mut().resize(4, 3);
        let new_area = Rect::new(0, 0, 4, 3);
        terminal.resize(new_area).unwrap();

        assert_eq!(terminal.viewport_area, new_area);
        assert_eq!(terminal.last_known_area, new_area);
        assert_eq!(terminal.buffers[terminal.current].area, new_area);
        assert_eq!(terminal.buffers[1 - terminal.current].area, new_area);
    }

    #[test]
    fn resize_fullscreen_triggers_clear_and_resets_back_buffer() {
        // This test is specifically about the side effects of `resize`:
        // - it calls `clear` to force a full redraw
        // - it resets the "previous" buffer
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        // Put visible content on the backend so we can tell whether a clear happened.
        {
            let frame = terminal.get_frame();
            frame.buffer[(0, 0)].set_symbol("x");
        }
        terminal.flush().unwrap();
        terminal.backend().assert_buffer_lines(["x  ", "   "]);

        terminal.backend_mut().resize(4, 3);
        let new_area = Rect::new(0, 0, 4, 3);
        terminal.resize(new_area).unwrap();

        terminal
            .backend()
            .assert_buffer_lines(["    ", "    ", "    "]);
        assert_eq!(
            terminal.buffers[1 - terminal.current],
            Buffer::empty(new_area)
        );
    }

    #[test]
    fn autoresize_fullscreen_uses_backend_size_when_changed() {
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        {
            let frame = terminal.get_frame();
            frame.buffer[(0, 0)].set_symbol("x");
        }
        terminal.flush().unwrap();

        terminal.backend_mut().resize(4, 3);
        terminal.autoresize().unwrap();

        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 4, 3));
        assert_eq!(terminal.last_known_area, Rect::new(0, 0, 4, 3));
        terminal
            .backend()
            .assert_buffer_lines(["    ", "    ", "    "]);
    }

    #[test]
    fn autoresize_fixed_does_not_change_viewport() {
        let backend = TestBackend::with_lines(["xxx", "yyy"]);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(1, 0, 2, 2)),
            },
        )
        .unwrap();

        terminal.autoresize().unwrap();

        assert_eq!(terminal.viewport_area, Rect::new(1, 0, 2, 2));
        assert_eq!(terminal.last_known_area, Rect::new(1, 0, 2, 2));
        terminal.backend().assert_buffer_lines(["xxx", "yyy"]);
    }

    #[test]
    fn resize_fixed_changes_viewport_area_and_buffer_sizes() {
        let backend = TestBackend::new(5, 3);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(1, 1, 2, 1)),
            },
        )
        .unwrap();

        terminal.resize(Rect::new(0, 0, 3, 2)).unwrap();

        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 3, 2));
        assert_eq!(terminal.last_known_area, Rect::new(0, 0, 3, 2));
        assert_eq!(
            terminal.buffers[terminal.current].area,
            terminal.viewport_area
        );
        assert_eq!(
            terminal.buffers[1 - terminal.current].area,
            terminal.viewport_area
        );
    }

    #[test]
    fn resize_inline_recomputes_origin_using_previous_cursor_offset() {
        let mut backend = TestBackend::new(10, 10);
        backend
            .set_cursor_position(Position { x: 0, y: 4 })
            .unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(4),
            },
        )
        .unwrap();

        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 10, 4));

        terminal.last_known_cursor_pos = Position { x: 0, y: 5 };
        terminal.backend_mut().resize(10, 12);
        let new_terminal_area = Rect::new(0, 0, 10, 12);
        terminal.resize(new_terminal_area).unwrap();

        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 10, 4));
        assert_eq!(terminal.last_known_area, new_terminal_area);
    }

    #[test]
    fn resize_inline_clamps_height_to_terminal_height() {
        let mut backend = TestBackend::new(10, 10);
        backend
            .set_cursor_position(Position { x: 0, y: 0 })
            .unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(10),
            },
        )
        .unwrap();

        terminal.backend_mut().resize(10, 3);
        terminal.resize(Rect::new(0, 0, 10, 3)).unwrap();

        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 10, 3));
    }

    #[test]
    fn resize_inline_preserves_backend_cursor_across_repeated_resizes() {
        let mut backend = TestBackend::new(10, 10);
        backend
            .set_cursor_position(Position { x: 0, y: 4 })
            .unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(4),
            },
        )
        .unwrap();

        terminal.resize(Rect::new(0, 0, 10, 12)).unwrap();
        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 10, 4));
        terminal.resize(Rect::new(0, 0, 10, 14)).unwrap();
        assert_eq!(terminal.viewport_area, Rect::new(0, 0, 10, 4));
        assert_eq!(
            terminal.backend().cursor_position(),
            Position { x: 0, y: 4 }
        );
    }

    // This tests for the case where the new width is smaller than the old
    // width. The screen should be cleared completely to avoid rendering
    // glitches caused by line wrap.
    #[test]
    fn resize_inline_clears_screen_on_horizontal_shrink() {
        let mut backend = TestBackend::with_lines(["0000", "1111"]);
        backend
            .set_cursor_position(Position { x: 0, y: 0 })
            .unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(2),
            },
        )
        .unwrap();

        let old_area = terminal.backend().buffer().area;
        let new_area = Rect {
            width: old_area.width - 1,
            ..old_area
        };

        terminal.resize(new_area);
        assert_eq!(terminal.viewport_area, new_area);
        let all_clear = terminal
            .current_buffer_mut()
            .content()
            .iter()
            .all(|cell| cell == &crate::buffer::Cell::EMPTY);
        assert!(all_clear, "not all buffer cells are empty");
    }

    #[test]
    fn resize_inline_bounds_cursor_movement_to_viewport_lines() {
        let mut backend = TestBackend::with_lines(["12345678", "12345678"]);
        backend
            .set_cursor_position(Position { x: 4, y: 1 })
            .unwrap();
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(2),
            },
        )
        .unwrap();

        terminal.inline_cursor_x = 4;
        terminal.inline_cursor_y = 1;

        // Shrink width from 8 to 4. Line 0 (8 chars) wraps into 2 rows (extra_rows = 1).
        // total_rows_up = inline_cursor_y (1) + extra_rows (1) + cursor_wrap (0) = 2.
        terminal.resize(Rect::new(0, 0, 4, 10)).unwrap();

        assert_eq!(terminal.inline_cursor_y, 0);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert_eq!(terminal.viewport_area.width, 4);
        assert!(terminal.force_full_redraw);
    }

    #[test]
    fn resize_inline_with_test_backend_preserves_scrollback_above_viewport() {
        let mut backend =
            TestBackend::with_lines(["echo foo  ", "foo       ", ">echo bar ", "suggestion"]);
        backend
            .set_cursor_position(Position { x: 4, y: 3 })
            .unwrap();

        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(2),
            },
        )
        .unwrap();

        terminal.inline_cursor_x = 4;
        terminal.inline_cursor_y = 1;

        // Draw initial frame at width 10
        terminal
            .draw(|frame| {
                let [line0, line1] = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .areas(frame.area());
                frame.render_widget(">echo bar ", line0);
                frame.render_widget("suggestion", line1);
            })
            .unwrap();

        // Shrink backend and terminal width from 10 to 5.
        terminal.backend_mut().resize(5, 10);
        terminal.resize(Rect::new(0, 0, 5, 10)).unwrap();

        assert_eq!(terminal.inline_cursor_y, 0);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert!(terminal.force_full_redraw);

        // Draw new frame at width 5
        terminal
            .draw(|frame| {
                let [line0, line1] = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .areas(frame.area());
                frame.render_widget(">echo", line0);
                frame.render_widget("sugg ", line1);
            })
            .unwrap();

        assert_eq!(terminal.viewport_area.width, 5);
    }

    #[test]
    fn resize_inline_30_cols_10_rows_verifies_buffer_before_and_after() {
        let backend = TestBackend::new(30, 10);

        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(6),
            },
        )
        .unwrap();

        // Draw initial frame at width 30 (scrollback in rows 0..3, active prompt starting 4 rows down at rows 4..5)
        terminal
            .draw(|frame| {
                let chunks = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .split(frame.area());

                frame.render_widget(
                    crate::text::Line::from("echo hello -------------------"),
                    chunks[0],
                );
                frame.render_widget(crate::text::Line::from("hello"), chunks[1]);
                frame.render_widget(crate::text::Line::from("echo world"), chunks[2]);
                frame.render_widget(crate::text::Line::from("world"), chunks[3]);
                frame.render_widget(
                    crate::text::Line::from(">echo 012345678901234567890123"),
                    chunks[4],
                );
                frame.render_widget(
                    crate::text::Line::from("suggestion_tooltip_here_text30"),
                    chunks[5],
                );
            })
            .unwrap();

        // Verify buffer state BEFORE resizing (width 30, height 10)
        // Scrollback history (rows 0..3) is intact, active prompt line starts 4 rows down (rows 4..5)
        terminal.backend().assert_buffer_lines([
            "echo hello -------------------",
            "hello                         ",
            "echo world                    ",
            "world                         ",
            ">echo 012345678901234567890123",
            "suggestion_tooltip_here_text30",
            "                              ",
            "                              ",
            "                              ",
            "                              ",
        ]);

        // Position cursor on viewport line 5 (row 5 of backend, 5 rows down)
        terminal.inline_cursor_y = 5;
        terminal.inline_cursor_x = 28;

        // Shrink backend and terminal width from 30 to 15 cols (height 10)
        terminal.backend_mut().resize(15, 10);
        terminal.resize(Rect::new(0, 0, 15, 10)).unwrap();

        assert_eq!(terminal.inline_cursor_y, 0);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert!(terminal.force_full_redraw);

        // Draw new frame reflowed to width 15
        terminal
            .draw(|frame| {
                let chunks = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .split(frame.area());

                frame.render_widget(crate::text::Line::from("echo hello ----"), chunks[0]);
                frame.render_widget(crate::text::Line::from("hello"), chunks[1]);
                frame.render_widget(crate::text::Line::from("echo world"), chunks[2]);
                frame.render_widget(crate::text::Line::from("world"), chunks[3]);
                frame.render_widget(crate::text::Line::from(">echo 012345678"), chunks[4]);
                frame.render_widget(crate::text::Line::from("suggestion_here"), chunks[5]);
            })
            .unwrap();

        // Verify buffer state AFTER resizing (width 15, height 10)
        // History and prompt are reflowed accurately!
        terminal.backend().assert_buffer_lines([
            "echo hello ----",
            "hello          ",
            "echo world     ",
            "world          ",
            ">echo 012345678",
            "suggestion_here",
            "               ",
            "               ",
            "               ",
            "               ",
        ]);
    }

    #[test]
    fn resize_inline_cursor_at_line_2_col_5_verifies_shrink_and_expand() {
        let backend = TestBackend::new(30, 10);

        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(5),
            },
        )
        .unwrap();

        // Draw initial frame at width 30
        terminal
            .draw(|frame| {
                let chunks = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .split(frame.area());

                frame.render_widget(
                    crate::text::Line::from("echo hello -------------------"),
                    chunks[0],
                );
                frame.render_widget(
                    crate::text::Line::from("prompt > 01234567890123456789"),
                    chunks[1],
                );
                frame.render_widget(crate::text::Line::from(">cat file.txt"), chunks[2]);
                frame.render_widget(crate::text::Line::from("suggestion 1"), chunks[3]);
                frame.render_widget(crate::text::Line::from("suggestion 2"), chunks[4]);
            })
            .unwrap();

        // Position cursor 2 lines into the viewport, 5 cols across
        terminal.inline_cursor_y = 2;
        terminal.inline_cursor_x = 5;

        // Verify buffer state BEFORE resizing (width 30, height 10)
        terminal.backend().assert_buffer_lines([
            "echo hello -------------------",
            "prompt > 01234567890123456789",
            ">cat file.txt                 ",
            "suggestion 1                  ",
            "suggestion 2                  ",
            "                              ",
            "                              ",
            "                              ",
            "                              ",
            "                              ",
        ]);

        // SHRINK width from 30 to 15 cols.
        // Line 0 (30 chars) wraps to 2 rows.
        // Line 1 (30 chars) wraps to 2 rows.
        // Cursor is at line 2, col 5. Total rows up = 2 + 2 = 4 rows.
        terminal.backend_mut().resize(15, 10);
        terminal.resize(Rect::new(0, 0, 15, 10)).unwrap();

        assert_eq!(terminal.inline_cursor_y, 0);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert!(terminal.force_full_redraw);

        // Draw new frame reflowed to width 15
        terminal
            .draw(|frame| {
                let chunks = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .split(frame.area());

                frame.render_widget(crate::text::Line::from("echo hello ----"), chunks[0]);
                frame.render_widget(crate::text::Line::from("prompt > 012345"), chunks[1]);
                frame.render_widget(crate::text::Line::from(">cat file.txt"), chunks[2]);
                frame.render_widget(crate::text::Line::from("suggestion 1"), chunks[3]);
                frame.render_widget(crate::text::Line::from("suggestion 2"), chunks[4]);
            })
            .unwrap();

        // Verify buffer state AFTER SHRINK (width 15, height 10)
        terminal.backend().assert_buffer_lines([
            "echo hello ----",
            "prompt > 012345",
            ">cat file.txt  ",
            "suggestion 1   ",
            "suggestion 2   ",
            "               ",
            "               ",
            "               ",
            "               ",
            "               ",
        ]);

        // Position cursor again 2 lines into the viewport, 5 cols across at width 15
        terminal.inline_cursor_y = 2;
        terminal.inline_cursor_x = 5;

        // EXPAND width back from 15 to 30 cols.
        // Line 0 (15 chars) takes 1 row at width 30.
        // Line 1 (15 chars) takes 1 row at width 30.
        // Cursor is at line 2, col 5. Total rows up = 1 + 1 = 2 rows.
        terminal.backend_mut().resize(30, 10);
        terminal.resize(Rect::new(0, 0, 30, 10)).unwrap();

        assert_eq!(terminal.inline_cursor_y, 0);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert!(terminal.force_full_redraw);

        // Draw new frame reflowed back to width 30
        terminal
            .draw(|frame| {
                let chunks = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .split(frame.area());

                frame.render_widget(
                    crate::text::Line::from("echo hello -------------------"),
                    chunks[0],
                );
                frame.render_widget(
                    crate::text::Line::from("prompt > 01234567890123456789"),
                    chunks[1],
                );
                frame.render_widget(crate::text::Line::from(">cat file.txt"), chunks[2]);
                frame.render_widget(crate::text::Line::from("suggestion 1"), chunks[3]);
                frame.render_widget(crate::text::Line::from("suggestion 2"), chunks[4]);
            })
            .unwrap();

        // Verify buffer state AFTER EXPAND (width 30, height 10)
        terminal.backend().assert_buffer_lines([
            "echo hello -------------------",
            "prompt > 01234567890123456789",
            ">cat file.txt                 ",
            "suggestion 1                  ",
            "suggestion 2                  ",
            "                              ",
            "                              ",
            "                              ",
            "                              ",
            "                              ",
        ]);
    }
}
