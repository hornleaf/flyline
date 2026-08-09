use std::error::Error;

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Paragraph, Widget};
use ratatui::{Terminal, TerminalOptions, Viewport};

#[test]
fn swap_buffer_clears_prev_buffer() {
    let backend = TestBackend::new(100, 50);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .current_buffer_mut()
        .set_string(0, 0, "Hello", ratatui::style::Style::reset());
    assert_eq!(terminal.current_buffer_mut().content()[0].symbol(), "H");
    terminal.swap_buffers();
    assert_eq!(terminal.current_buffer_mut().content()[0].symbol(), " ");
}

#[test]
fn terminal_draw_returns_the_completed_frame() -> Result<(), Box<dyn Error>> {
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend)?;
    let frame = terminal.draw(|f| {
        let paragraph = Paragraph::new("Test");
        f.render_widget(paragraph, f.area());
    })?;
    assert_eq!(frame.buffer[(0, 0)].symbol(), "T");
    assert_eq!(frame.area, Rect::new(0, 0, 10, 10));
    terminal.backend_mut().resize(8, 8);
    let frame = terminal.draw(|f| {
        let paragraph = Paragraph::new("test");
        f.render_widget(paragraph, f.area());
    })?;
    assert_eq!(frame.buffer[(0, 0)].symbol(), "t");
    assert_eq!(frame.area, Rect::new(0, 0, 8, 8));
    Ok(())
}

#[test]
fn terminal_draw_increments_frame_count() -> Result<(), Box<dyn Error>> {
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend)?;
    let frame = terminal.draw(|f| {
        assert_eq!(f.count(), 0);
        let paragraph = Paragraph::new("Test");
        f.render_widget(paragraph, f.area());
    })?;
    assert_eq!(frame.count, 0);
    let frame = terminal.draw(|f| {
        assert_eq!(f.count(), 1);
        let paragraph = Paragraph::new("test");
        f.render_widget(paragraph, f.area());
    })?;
    assert_eq!(frame.count, 1);
    let frame = terminal.draw(|f| {
        assert_eq!(f.count(), 2);
        let paragraph = Paragraph::new("test");
        f.render_widget(paragraph, f.area());
    })?;
    assert_eq!(frame.count, 2);
    Ok(())
}
