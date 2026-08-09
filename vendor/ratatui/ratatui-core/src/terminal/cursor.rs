use crate::backend::Backend;
use crate::layout::Position;
use crate::terminal::{Terminal, Viewport};

impl<B: Backend> Terminal<B> {
    /// Hides the cursor.
    ///
    /// When using [`Terminal::draw`] / [`Terminal::try_draw`], prefer controlling the cursor with
    /// [`Frame::set_cursor_position`]. A later successful [`Terminal::draw`] /
    /// [`Terminal::try_draw`] call may overwrite this change.
    ///
    /// [`Frame::set_cursor_position`]: crate::terminal::Frame::set_cursor_position
    /// [`Terminal::draw`]: crate::terminal::Terminal::draw
    /// [`Terminal::try_draw`]: crate::terminal::Terminal::try_draw
    pub fn hide_cursor(&mut self) -> Result<(), B::Error> {
        self.backend.hide_cursor()?;
        self.hidden_cursor = true;
        Ok(())
    }

    /// Shows the cursor.
    ///
    /// When using [`Terminal::draw`] / [`Terminal::try_draw`], prefer controlling the cursor with
    /// [`Frame::set_cursor_position`]. A later successful [`Terminal::draw`] /
    /// [`Terminal::try_draw`] call may overwrite this change.
    ///
    /// [`Frame::set_cursor_position`]: crate::terminal::Frame::set_cursor_position
    /// [`Terminal::draw`]: crate::terminal::Terminal::draw
    /// [`Terminal::try_draw`]: crate::terminal::Terminal::try_draw
    pub fn show_cursor(&mut self) -> Result<(), B::Error> {
        self.backend.show_cursor()?;
        self.hidden_cursor = false;
        Ok(())
    }

    /// Gets the current cursor position.
    ///
    /// This queries the backend for the current cursor position and returns it as an `(x, y)`
    /// tuple.
    #[deprecated = "use `get_cursor_position()` instead which returns `Result<Position>`"]
    pub fn get_cursor(&mut self) -> Result<(u16, u16), B::Error> {
        let Position { x, y } = self.get_cursor_position()?;
        Ok((x, y))
    }

    /// Sets the cursor position.
    #[deprecated = "use `set_cursor_position((x, y))` instead which takes `impl Into<Position>`"]
    pub fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), B::Error> {
        self.set_cursor_position(Position { x, y })
    }

    /// Gets the current cursor position.
    ///
    /// This queries the backend for the current cursor position. It is not limited to Ratatui's
    /// last render pass, so direct backend mutations may also affect the returned value.
    ///
    /// When using [`Terminal::draw`] / [`Terminal::try_draw`], prefer controlling the cursor with
    /// [`Frame::set_cursor_position`]. For direct control, see [`Terminal::set_cursor_position`].
    ///
    /// [`Frame::set_cursor_position`]: crate::terminal::Frame::set_cursor_position
    /// [`Terminal::draw`]: crate::terminal::Terminal::draw
    /// [`Terminal::try_draw`]: crate::terminal::Terminal::try_draw
    pub fn get_cursor_position(&mut self) -> Result<Position, B::Error> {
        self.backend.get_cursor_position()
    }

    /// Sets the cursor position.
    ///
    /// This updates the backend cursor and Ratatui's internal cursor tracking. Inline viewports
    /// use that tracking when recomputing the viewport on resize.
    ///
    /// When using [`Terminal::draw`] / [`Terminal::try_draw`], consider using
    /// [`Frame::set_cursor_position`] instead so the cursor is updated as part of the normal
    /// rendering flow. A later successful
    /// [`Terminal::draw`] / [`Terminal::try_draw`] call may overwrite a direct cursor move.
    ///
    /// [`Frame::set_cursor_position`]: crate::terminal::Frame::set_cursor_position
    /// [`Terminal::draw`]: crate::terminal::Terminal::draw
    /// [`Terminal::try_draw`]: crate::terminal::Terminal::try_draw
    pub fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), B::Error> {
        let position = position.into();
        if matches!(self.viewport, Viewport::Inline(_)) {
            if let Some(top) = self.viewport_top {
                // The viewport's on-screen origin is known: use absolute
                // positioning, translating the viewport-relative row by the
                // viewport top.  Relative moves drift whenever a line wraps or
                // a wide grapheme (CJK, emoji) advances the terminal cursor by
                // two columns, which makes every frame scroll down one line.
                self.backend.set_cursor_position(Position {
                    x: position.x,
                    y: position.y + top,
                })?;
            } else {
                // The on-screen origin is not known yet (e.g. before the first
                // cursor-position report); fall back to the original relative
                // moves, which are correct as long as no drift has accumulated.
                let dy = position.y as i32 - self.inline_cursor_y as i32;
                if dy != 0 {
                    self.backend.move_cursor_relative(0, dy as i16)?;
                }
                if self.inline_cursor_x > 0 {
                    self.backend
                        .move_cursor_relative(-(self.inline_cursor_x as i16), 0)?;
                }
                if position.x > 0 {
                    self.backend.move_cursor_relative(position.x as i16, 0)?;
                }
            }
            self.inline_cursor_x = position.x;
            self.inline_cursor_y = position.y;
        } else {
            self.backend.set_cursor_position(position)?;
        }
        self.last_known_cursor_pos = position;
        Ok(())
    }

    /// Resets relative in-memory cursor tracking to (0, 0).
    pub fn reset_inline_cursor(&mut self) {
        self.inline_cursor_x = 0;
        self.inline_cursor_y = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::{Backend, TestBackend};
    use crate::layout::Position;
    use crate::terminal::Terminal;

    #[test]
    fn hide_cursor_updates_terminal_state() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.hide_cursor().unwrap();

        assert!(terminal.hidden_cursor);
        assert!(!terminal.backend().cursor_visible());
    }

    #[test]
    fn show_cursor_updates_terminal_state() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.hide_cursor().unwrap();
        terminal.show_cursor().unwrap();

        assert!(!terminal.hidden_cursor);
        assert!(terminal.backend().cursor_visible());
    }

    #[test]
    fn set_cursor_position_updates_backend_and_tracking() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.set_cursor_position((3, 4)).unwrap();

        assert_eq!(terminal.last_known_cursor_pos, Position { x: 3, y: 4 });
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 3, y: 4 });
    }

    #[test]
    fn get_cursor_position_queries_backend() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .backend_mut()
            .set_cursor_position(Position { x: 7, y: 2 })
            .unwrap();

        assert_eq!(
            terminal.get_cursor_position().unwrap(),
            Position { x: 7, y: 2 }
        );
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_cursor_wrappers_delegate_to_position_apis() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.set_cursor(4, 1).unwrap();

        assert_eq!(terminal.get_cursor().unwrap(), (4, 1));
        assert_eq!(terminal.last_known_cursor_pos, Position { x: 4, y: 1 });
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 4, y: 1 });
    }
}
