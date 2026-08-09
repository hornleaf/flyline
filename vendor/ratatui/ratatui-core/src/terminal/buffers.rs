use crate::backend::{Backend, ClearType};
use crate::buffer::{Buffer, Cell, CellWidth};
use crate::layout::{Position, Rect};
use crate::terminal::{Frame, Terminal, Viewport};

impl<B: Backend> Terminal<B> {
    /// Returns a [`Frame`] for manual rendering.
    ///
    /// Most applications should render via [`Terminal::draw`] / [`Terminal::try_draw`]. This is an
    /// escape hatch that exposes the frame construction step used by [`Terminal::try_draw`] so
    /// tests and advanced callers can render without running the full draw pipeline.
    ///
    /// This is primarily useful for tests, backend adapters, and specialized integrations that
    /// intentionally manage presentation themselves.
    ///
    /// Unlike `draw` / `try_draw`, this does not call [`Terminal::autoresize`], does not write
    /// updates to the backend, and does not apply any cursor changes. After rendering, you
    /// typically call [`Terminal::flush`], [`Terminal::swap_buffers`], and [`Backend::flush`].
    ///
    /// For the full render-pass behavior that also handles resizing, cursor updates, buffer
    /// swapping, and backend flushing, see [`Terminal::draw`] and [`Terminal::try_draw`].
    ///
    /// The returned `Frame` mutably borrows the current buffer, so it must be dropped before you
    /// can call methods like [`Terminal::flush`]. The example below uses a scope to make that
    /// explicit.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # mod ratatui {
    /// #     pub use ratatui_core::backend;
    /// #     pub use ratatui_core::terminal::Terminal;
    /// # }
    /// use ratatui::Terminal;
    /// use ratatui::backend::{Backend, TestBackend};
    ///
    /// let backend = TestBackend::new(30, 5);
    /// let mut terminal = Terminal::new(backend)?;
    /// {
    ///     let mut frame = terminal.get_frame();
    ///     frame.render_widget("Hello", frame.area());
    /// }
    /// // When not using `draw`, present the buffer manually:
    /// terminal.flush()?;
    /// terminal.swap_buffers();
    /// terminal.backend_mut().flush()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`Backend::flush`]: crate::backend::Backend::flush
    pub const fn get_frame(&mut self) -> Frame<'_> {
        let count = self.frame_count;
        Frame {
            cursor_position: None,
            viewport_area: self.viewport_area,
            buffer: self.current_buffer_mut(),
            count,
        }
    }

    /// Gets the current buffer as a mutable reference.
    ///
    /// This is the buffer that the next [`Frame`] will render into (see [`Terminal::get_frame`]).
    /// This is a low-level escape hatch; normal applications should render inside
    /// [`Terminal::draw`] and access the buffer through widgets, or through [`Frame::buffer_mut`]
    /// when they intentionally need direct cell access during a render pass.
    ///
    /// Mutating this buffer does not update the backend immediately. The changes become visible
    /// only after a later [`Terminal::flush`] or full draw pass applies the diff. Because this
    /// bypasses the usual render callback structure, it is mainly useful for tests and specialized
    /// integrations that intentionally manage presentation themselves.
    pub const fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    /// Applies the current buffer diff to the backend's active display surface.
    ///
    /// This compares the current buffer with the previous buffer and passes only the changed cells
    /// to [`Backend::draw`]. It is one of the building blocks used by [`Terminal::draw`] /
    /// [`Terminal::try_draw`].
    ///
    /// This method does not swap buffers, does not update cursor visibility or position, and does
    /// not call [`Backend::flush`]. See [`Terminal::swap_buffers`] and [`Backend::flush`].
    ///
    /// `Terminal::flush` only reasons about Ratatui's internal buffers. It does not know whether
    /// the backend's display surface changed since the last render pass. For example, if you leave
    /// the alternate screen and then call `Terminal::flush`, Ratatui may replay a diff that was
    /// computed for the alternate screen onto the main screen. In normal applications, prefer
    /// [`Terminal::draw`] / [`Terminal::try_draw`] unless you are intentionally managing the whole
    /// render pipeline yourself.
    ///
    /// Implementation note: when there are updates, Ratatui records the position of the last
    /// updated cell as the "last known cursor position". Inline viewports use this to preserve the
    /// cursor's relative position within the viewport across resizes.
    ///
    /// [`Backend::flush`]: crate::backend::Backend::flush
    pub fn flush(&mut self) -> Result<(), B::Error> {
        if matches!(self.viewport, Viewport::Inline(_)) {
            return self.flush_inline();
        }

        let previous_buffer = &self.buffers[1 - self.current];
        let current_buffer = &self.buffers[self.current];
        let mut last_pos = None;

        let updates = previous_buffer
            .diff_iter(current_buffer)
            .inspect(|(col, row, _)| {
                last_pos = Some(Position { x: *col, y: *row });
            });
        self.backend.draw(updates)?;

        if let Some(pos) = last_pos {
            self.last_known_cursor_pos = pos;
        }

        Ok(())
    }

    fn flush_inline(&mut self) -> Result<(), B::Error> {
        // Keep the viewport inside the terminal: when the content is taller
        // than the space below the viewport origin, shift the origin up so the
        // whole viewport remains visible instead of being truncated at the
        // bottom of the screen.
        self.update_viewport_top_for_height();

        let width = self.viewport_area.width;
        let height = self.viewport_area.height;
        let current = self.current;
        let force_redraw = self.force_full_redraw;
        self.force_full_redraw = false;

        for i in 0..height {
            let line_changed = force_redraw
                || (0..width).any(|col| {
                    self.buffers[1 - current][(col, i)] != self.buffers[current][(col, i)]
                });
            if line_changed {
                self.set_cursor_position(Position { x: 0, y: i })?;

                let mut last_col = None;
                for col in (0..width).rev() {
                    let cell = &self.buffers[current][(col, i)];
                    let is_whitespace = (cell.symbol() == " " || cell.symbol().is_empty())
                        && cell.style() == crate::style::Style::default();
                    if !is_whitespace {
                        last_col = Some(col);
                        break;
                    }
                }

                if let Some(last) = last_col {
                    let line_cells =
                        (0..=last).map(|col| (col, i, &self.buffers[current][(col, i)]));
                    self.backend.draw_relative_line(line_cells)?;

                    // Compute the real terminal width of the emitted line.
                    // Wide graphemes occupy two terminal columns while their
                    // grid coordinate advances by one; the backend skips their
                    // empty continuation cells, so those contribute nothing.
                    let mut real_width: u16 = 0;
                    let mut prev_cell_width: u16 = 1;
                    for col in 0..=last {
                        let cell = &self.buffers[current][(col, i)];
                        let w = cell.cell_width() as u16;
                        if cell.symbol().is_empty() {
                            if prev_cell_width > 1 {
                                // Continuation cell of a wide grapheme.
                                continue;
                            }
                            // The backend renders an empty cell as a space.
                            real_width += 1;
                            prev_cell_width = 1;
                        } else {
                            real_width += w;
                            prev_cell_width = w;
                        }
                    }

                    self.inline_cursor_x = real_width;
                    if real_width < width {
                        self.set_cursor_position(Position { x: real_width, y: i })?;
                        self.backend.clear_region(ClearType::UntilNewLine)?;
                    }
                } else {
                    self.set_cursor_position(Position { x: 0, y: i })?;
                    self.backend.clear_region(ClearType::UntilNewLine)?;
                    self.inline_cursor_x = width;
                }
            }
        }
        Ok(())
    }

    /// Clears the inactive buffer and swaps it with the current buffer.
    ///
    /// This is part of the standard rendering flow (see [`Terminal::try_draw`]). If you render
    /// manually using [`Terminal::get_frame`] and [`Terminal::flush`], call this immediately
    /// afterward so the next flush can compute diffs against the correct "previous" buffer.
    pub fn swap_buffers(&mut self) {
        self.buffers[1 - self.current].reset();
        self.current = 1 - self.current;
    }

    /// Clear the terminal and force a full redraw on the next draw call.
    ///
    /// What gets cleared depends on the active [`Viewport`]:
    ///
    /// - [`Viewport::Fullscreen`]: clears the entire terminal.
    /// - [`Viewport::Fixed`]: clears only the viewport region.
    /// - [`Viewport::Inline`]: clears after the viewport's origin, leaving any content above the
    ///   viewport untouched.
    ///
    /// Current behavior: for [`Viewport::Inline`], clearing runs from the viewport origin through
    /// the bottom of the viewport area.
    ///
    /// This also resets the "previous" buffer so the next [`Terminal::flush`] redraws the full
    /// viewport.
    ///
    /// [`Terminal::resize`]: crate::terminal::Terminal::resize
    ///
    /// Implementation note: this uses [`ClearType::AfterCursor`] starting at the viewport origin.
    pub fn clear(&mut self) -> Result<(), B::Error> {
        if matches!(self.viewport, Viewport::Inline(_)) {
            self.clear_viewport()?;
            self.backend.clear_region(ClearType::All)?;
            self.set_cursor_position(Position::ORIGIN)?;
            self.inline_cursor_x = 0;
            self.inline_cursor_y = 0;
        } else {
            let original_cursor = self.backend.get_cursor_position()?;
            self.clear_viewport()?;
            self.backend.set_cursor_position(original_cursor)?;
        }
        Ok(())
    }

    /// Clears according to the current viewport and resets the back buffer.
    ///
    /// Unlike [`Terminal::clear`], this does not snapshot and restore the backend cursor
    /// position. Callers that need to preserve the cursor should do so outside this helper.
    pub(super) fn clear_viewport(&mut self) -> Result<(), B::Error> {
        match self.viewport {
            Viewport::Fullscreen => self.backend.clear_region(ClearType::All)?,
            Viewport::Inline(_) => {
                // Reposition at the viewport origin first (absolute when the
                // on-screen origin is known) instead of moving relative to the
                // tracked cursor, which may have drifted.
                self.set_cursor_position(Position::ORIGIN)?;
                self.backend.clear_region(ClearType::AfterCursor)?;
            }
            Viewport::Fixed(_) => {
                let area = self.viewport_area;
                self.clear_fixed_viewport(area)?;
            }
        }
        // Reset both buffers and force a full redraw on the next update.
        self.buffers[0].reset();
        self.buffers[1].reset();
        self.force_full_redraw = true;
        Ok(())
    }

    /// Clears a fixed viewport using terminal clear commands when possible.
    ///
    /// Terminal clear commands can be faster than per-cell updates.
    fn clear_fixed_viewport(&mut self, area: Rect) -> Result<(), B::Error> {
        if area.is_empty() {
            return Ok(());
        }
        let size = self.backend.size()?;
        let is_full_width = area.x == 0 && area.width == size.width;
        let ends_at_bottom = area.bottom() == size.height;
        if is_full_width && ends_at_bottom {
            self.backend.set_cursor_position(area.as_position())?;
            self.backend.clear_region(ClearType::AfterCursor)?;
        } else if is_full_width {
            self.clear_full_width_rows(area)?;
        } else {
            self.clear_region_cells(area)?;
        }
        Ok(())
    }

    /// Clears full-width rows using line clear commands.
    ///
    /// This avoids per-cell writes when the viewport spans the full width.
    fn clear_full_width_rows(&mut self, area: Rect) -> Result<(), B::Error> {
        for y in area.top()..area.bottom() {
            self.backend.set_cursor_position(Position { x: 0, y })?;
            self.backend.clear_region(ClearType::CurrentLine)?;
        }
        Ok(())
    }

    /// Clears a non-full-width region by writing empty cells directly.
    ///
    /// This is used when line-based clears would affect cells outside the viewport.
    fn clear_region_cells(&mut self, area: Rect) -> Result<(), B::Error> {
        let clear_cell = Cell::default();
        let updates = area.positions().map(|pos| (pos.x, pos.y, &clear_cell));
        self.backend.draw(updates)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::{Backend, TestBackend};
    use crate::buffer::{Buffer, Cell};
    use crate::layout::{Position, Rect};
    use crate::terminal::{Terminal, TerminalOptions, Viewport};

    #[test]
    fn get_frame_uses_current_viewport_and_frame_count() {
        let backend = TestBackend::new(5, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        let frame = terminal.get_frame();
        assert_eq!(frame.count, 0);
        assert_eq!(frame.area().width, 5);
        assert_eq!(frame.area().height, 3);
        assert_eq!(frame.buffer.area, frame.area());
    }

    #[test]
    fn flush_writes_updates_and_tracks_last_updated_cell() {
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        {
            let frame = terminal.get_frame();
            frame.buffer[(1, 0)].set_symbol("x");
        }

        terminal.flush().unwrap();
        terminal.backend().assert_buffer_lines([" x ", "   "]);
        assert_eq!(terminal.last_known_cursor_pos, Position { x: 1, y: 0 });
    }

    #[test]
    fn flush_with_no_updates_does_not_change_last_known_cursor_pos() {
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.set_cursor_position((2, 1)).unwrap();

        terminal.flush().unwrap();

        assert_eq!(terminal.last_known_cursor_pos, Position { x: 2, y: 1 });
    }

    #[test]
    fn swap_buffers_resets_new_current_buffer() {
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.buffers[1][(0, 0)].set_symbol("x");
        terminal.swap_buffers();

        assert_eq!(terminal.current, 1);
        assert_eq!(
            terminal.buffers[terminal.current],
            Buffer::empty(terminal.viewport_area)
        );
    }

    #[test]
    fn clear_fullscreen_clears_backend_and_resets_back_buffer() {
        let backend = TestBackend::new(3, 2);
        let mut terminal = Terminal::new(backend).unwrap();

        {
            let frame = terminal.get_frame();
            frame.buffer[(0, 0)] = Cell::new("x");
        }
        terminal.flush().unwrap();
        terminal.backend().assert_buffer_lines(["x  ", "   "]);

        terminal.buffers[1][(2, 1)] = Cell::new("y");
        terminal.clear().unwrap();

        terminal.backend().assert_buffer_lines(["   ", "   "]);
        assert_eq!(
            terminal.buffers[1 - terminal.current],
            Buffer::empty(terminal.viewport_area)
        );
    }

    #[test]
    fn clear_inline_clears_after_viewport_origin_and_resets_back_buffer() {
        // Inline clear is implemented as:
        //   1) move the backend cursor to the viewport origin
        //   2) call ClearType::AfterCursor once
        let mut backend = TestBackend::with_lines([
            "before 1  ",
            "before 2  ",
            "viewport 1",
            "viewport 2",
            "after 1   ",
            "after 2   ",
        ]);
        backend
            .set_cursor_position(Position { x: 2, y: 2 })
            .unwrap();
        let options = TerminalOptions {
            viewport: Viewport::Inline(2),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();
        terminal
            .backend_mut()
            .set_cursor_position(Position { x: 2, y: 2 })
            .unwrap();

        terminal.buffers[1][(2, 1)] = Cell::new("x");
        terminal.clear().unwrap();

        assert_eq!(
            terminal.buffers[1 - terminal.current],
            Buffer::empty(terminal.viewport_area)
        );
    }

    #[test]
    fn clear_fixed_clears_viewport_rows_and_resets_back_buffer() {
        // For full-width fixed viewports that reach the terminal bottom, clear uses
        // ClearType::AfterCursor starting at the viewport origin.
        let mut backend = TestBackend::with_lines(["before 1  ", "viewport 1", "viewport 2"]);
        backend.set_cursor_position((2, 0)).unwrap();
        let options = TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 1, 10, 2)),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        terminal.clear().unwrap();

        terminal
            .backend()
            .assert_buffer_lines(["before 1  ", "          ", "          "]);
        assert_eq!(
            terminal.buffers[1 - terminal.current],
            Buffer::empty(terminal.viewport_area)
        );
        assert_eq!(
            terminal.backend().cursor_position(),
            Position { x: 2, y: 0 }
        );
    }

    #[test]
    fn clear_fixed_full_width_not_at_bottom() {
        let mut backend =
            TestBackend::with_lines(["before 1  ", "viewport 1", "viewport 2", "after 1   "]);
        backend.set_cursor_position((1, 0)).unwrap();
        let options = TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 1, 10, 2)),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        terminal.clear().unwrap();

        terminal.backend().assert_buffer_lines([
            "before 1  ",
            "          ",
            "          ",
            "after 1   ",
        ]);
        assert_eq!(
            terminal.backend().cursor_position(),
            Position { x: 1, y: 0 }
        );
    }

    #[test]
    fn clear_fixed_respects_non_full_width_viewport() {
        let mut backend =
            TestBackend::with_lines(["before 1  ", "viewport 1", "viewport 2", "after 1   "]);
        backend.set_cursor_position((3, 0)).unwrap();
        let options = TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(1, 1, 3, 2)),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        terminal.clear().unwrap();

        terminal.backend().assert_buffer_lines([
            "before 1  ",
            "v   port 1",
            "v   port 2",
            "after 1   ",
        ]);
        assert_eq!(
            terminal.backend().cursor_position(),
            Position { x: 3, y: 0 }
        );
    }

    #[test]
    fn clear_viewport_inline_leaves_cursor_at_viewport_origin() {
        let mut backend = TestBackend::with_lines([
            "before 1  ",
            "before 2  ",
            "viewport 1",
            "viewport 2",
            "after 1   ",
            "after 2   ",
        ]);
        backend
            .set_cursor_position(Position { x: 2, y: 2 })
            .unwrap();
        let options = TerminalOptions {
            viewport: Viewport::Inline(2),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        terminal.clear_viewport().unwrap();

        assert_eq!(terminal.backend().cursor_position(), Position::new(0, 2));
    }

    #[test]
    fn clear_terminal_inline_resets_cursor_to_origin_and_forces_full_redraw() {
        let backend = TestBackend::new(10, 3);
        let options = TerminalOptions {
            viewport: Viewport::Inline(3),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        terminal
            .draw(|frame| {
                frame.render_widget("Hello", frame.area());
            })
            .unwrap();

        terminal.clear().unwrap();

        assert_eq!(terminal.backend().cursor_position(), Position::ORIGIN);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert_eq!(terminal.inline_cursor_y, 0);
        assert!(terminal.force_full_redraw);

        // Verify next draw pass re-renders all lines completely
        terminal
            .draw(|frame| {
                frame.render_widget("World", frame.area());
            })
            .unwrap();

        terminal
            .backend()
            .assert_buffer_lines(["World     ", "          ", "          "]);
    }

    #[test]
    fn clear_terminal_inline_when_cursor_started_at_non_zero_row() {
        let mut backend =
            TestBackend::with_lines(["line 0    ", "line 1    ", "line 2    ", "line 3    "]);
        backend
            .set_cursor_position(Position { x: 3, y: 2 })
            .unwrap();

        let options = TerminalOptions {
            viewport: Viewport::Inline(2),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        terminal.clear().unwrap();

        assert_eq!(terminal.backend().cursor_position(), Position::ORIGIN);
        assert_eq!(terminal.inline_cursor_x, 0);
        assert_eq!(terminal.inline_cursor_y, 0);
    }

    #[test]
    fn flush_inline_does_not_print_spaces_when_clearing_line_to_blank() {
        let backend = TestBackend::new(10, 2);
        let options = TerminalOptions {
            viewport: Viewport::Inline(2),
        };
        let mut terminal = Terminal::with_options(backend, options).unwrap();

        // Frame 1: render text on line 1
        terminal
            .draw(|frame| {
                let [_, line1] = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .areas(frame.area());
                frame.render_widget("Old Text", line1);
            })
            .unwrap();

        // Frame 2: line 1 is now cleared to blank
        terminal
            .draw(|frame| {
                let [line0, _] = crate::layout::Layout::vertical([
                    crate::layout::Constraint::Length(1),
                    crate::layout::Constraint::Length(1),
                ])
                .areas(frame.area());
                frame.render_widget("Hi", line0);
            })
            .unwrap();

        terminal
            .backend()
            .assert_buffer_lines(["Hi        ", "          "]);
        assert_eq!(terminal.inline_cursor_x, 10);
        assert_eq!(terminal.inline_cursor_y, 1);
        assert_eq!(terminal.backend().cursor_position(), Position::new(10, 1));
    }
}
