use vte::{Params, Perform};

/// 24-bit color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Default foreground (white).
    pub const fn default_fg() -> Self {
        Self { r: 255, g: 255, b: 255 }
    }

    /// Default background (black).
    pub const fn default_bg() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    /// Convert from 256-color index.
    pub fn from_256(idx: u8) -> Self {
        match idx {
            0 => Color::rgb(0, 0, 0),
            1 => Color::rgb(128, 0, 0),
            2 => Color::rgb(0, 128, 0),
            3 => Color::rgb(128, 128, 0),
            4 => Color::rgb(0, 0, 128),
            5 => Color::rgb(128, 0, 128),
            6 => Color::rgb(0, 128, 128),
            7 => Color::rgb(192, 192, 192),
            8 => Color::rgb(128, 128, 128),
            9 => Color::rgb(255, 0, 0),
            10 => Color::rgb(0, 255, 0),
            11 => Color::rgb(255, 255, 0),
            12 => Color::rgb(0, 0, 255),
            13 => Color::rgb(255, 0, 255),
            14 => Color::rgb(0, 255, 255),
            15 => Color::rgb(255, 255, 255),
            // 216-color cube (indices 16-231)
            16..=231 => {
                let idx = idx - 16;
                let b = idx % 6;
                let g = (idx / 6) % 6;
                let r = idx / 36;
                let to_val = |c: u8| if c == 0 { 0 } else { 55 + 40 * c };
                Color::rgb(to_val(r), to_val(g), to_val(b))
            }
            // Grayscale (indices 232-255)
            232..=255 => {
                let v = 8 + 10 * (idx - 232);
                Color::rgb(v, v, v)
            }
        }
    }
}

/// A single terminal cell.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::default_fg(),
            bg: Color::default_bg(),
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

/// SGR (Select Graphic Rendition) state.
#[derive(Debug, Clone)]
struct SgrState {
    fg: Color,
    bg: Color,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

impl Default for SgrState {
    fn default() -> Self {
        Self {
            fg: Color::default_fg(),
            bg: Color::default_bg(),
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

/// Virtual terminal: grid of cells with cursor and SGR state.
pub struct VirtualTerminal {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
    cursor_row: usize,
    cursor_col: usize,
    sgr: SgrState,
    saved_cursor: Option<(usize, usize)>,
    /// Whether the alternate screen buffer is active.
    pub alt_screen: bool,
    /// Flag set when sync-end (CSI ?2026l) is received.
    pub sync_end_seen: bool,
}

impl VirtualTerminal {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::default(); width]; height];
        Self {
            width,
            height,
            cells,
            cursor_row: 0,
            cursor_col: 0,
            sgr: SgrState::default(),
            saved_cursor: None,
            alt_screen: false,
            sync_end_seen: false,
        }
    }

    /// Feed raw bytes through the ANSI parser.
    pub fn feed(&mut self, data: &[u8]) {
        let mut parser = vte::Parser::new();
        for &byte in data {
            parser.advance(self, byte);
        }
    }

    /// Get plain text of a row.
    pub fn row_text(&self, row: usize) -> String {
        if row >= self.height {
            return String::new();
        }
        self.cells[row].iter().map(|c| c.ch).collect()
    }

    /// Get all rows as plain text.
    pub fn all_text(&self) -> Vec<String> {
        (0..self.height).map(|r| self.row_text(r)).collect()
    }

    /// Resize the terminal grid.
    pub fn resize(&mut self, width: usize, height: usize) {
        let mut new_cells = vec![vec![Cell::default(); width]; height];
        for r in 0..height.min(self.height) {
            for c in 0..width.min(self.width) {
                new_cells[r][c] = self.cells[r][c].clone();
            }
        }
        self.cells = new_cells;
        self.width = width;
        self.height = height;
        self.cursor_row = self.cursor_row.min(height.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(width.saturating_sub(1));
    }

    /// Clear the entire grid.
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_row < self.height && self.cursor_col < self.width {
            let cell = &mut self.cells[self.cursor_row][self.cursor_col];
            cell.ch = ch;
            cell.fg = self.sgr.fg;
            cell.bg = self.sgr.bg;
            cell.bold = self.sgr.bold;
            cell.dim = self.sgr.dim;
            cell.italic = self.sgr.italic;
            cell.underline = self.sgr.underline;
            cell.strikethrough = self.sgr.strikethrough;
            self.cursor_col += 1;
            if self.cursor_col >= self.width {
                self.cursor_col = self.width - 1;
            }
        }
    }

    fn erase_in_display(&mut self, mode: u16) {
        match mode {
            // Clear from cursor to end of screen.
            0 => {
                // Clear rest of current row.
                for c in self.cursor_col..self.width {
                    self.cells[self.cursor_row][c] = Cell::default();
                }
                // Clear all subsequent rows.
                for r in (self.cursor_row + 1)..self.height {
                    for c in 0..self.width {
                        self.cells[r][c] = Cell::default();
                    }
                }
            }
            // Clear from start of screen to cursor.
            1 => {
                for r in 0..self.cursor_row {
                    for c in 0..self.width {
                        self.cells[r][c] = Cell::default();
                    }
                }
                for c in 0..=self.cursor_col.min(self.width - 1) {
                    self.cells[self.cursor_row][c] = Cell::default();
                }
            }
            // Clear entire screen.
            2 | 3 => self.clear(),
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: u16) {
        if self.cursor_row >= self.height {
            return;
        }
        match mode {
            // Clear from cursor to end of line.
            0 => {
                for c in self.cursor_col..self.width {
                    self.cells[self.cursor_row][c] = Cell::default();
                }
            }
            // Clear from start of line to cursor.
            1 => {
                for c in 0..=self.cursor_col.min(self.width - 1) {
                    self.cells[self.cursor_row][c] = Cell::default();
                }
            }
            // Clear entire line.
            2 => {
                for c in 0..self.width {
                    self.cells[self.cursor_row][c] = Cell::default();
                }
            }
            _ => {}
        }
    }

    /// Parse SGR parameters. Handles 38;2;r;g;b, 48;2;r;g;b, 38;5;n, 48;5;n, and basic codes.
    fn apply_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();
        while let Some(param) = iter.next() {
            let code = param[0];
            match code {
                0 => self.sgr = SgrState::default(),
                1 => self.sgr.bold = true,
                2 => self.sgr.dim = true,
                3 => self.sgr.italic = true,
                4 => self.sgr.underline = true,
                9 => self.sgr.strikethrough = true,
                22 => {
                    self.sgr.bold = false;
                    self.sgr.dim = false;
                }
                23 => self.sgr.italic = false,
                24 => self.sgr.underline = false,
                25 => {} // blink off (no-op for us)
                29 => self.sgr.strikethrough = false,
                // Standard foreground colors (30-37)
                30..=37 => {
                    self.sgr.fg = ansi_basic_color(code - 30);
                }
                38 => {
                    // Extended foreground.
                    if let Some(sub) = iter.next() {
                        match sub[0] {
                            2 => {
                                // 24-bit: 38;2;r;g;b
                                let r = iter.next().map_or(0, |p| p[0] as u8);
                                let g = iter.next().map_or(0, |p| p[0] as u8);
                                let b = iter.next().map_or(0, |p| p[0] as u8);
                                self.sgr.fg = Color::rgb(r, g, b);
                            }
                            5 => {
                                // 256-color: 38;5;n
                                let n = iter.next().map_or(0, |p| p[0] as u8);
                                self.sgr.fg = Color::from_256(n);
                            }
                            _ => {}
                        }
                    }
                }
                39 => self.sgr.fg = Color::default_fg(),
                // Standard background colors (40-47)
                40..=47 => {
                    self.sgr.bg = ansi_basic_color(code - 40);
                }
                48 => {
                    // Extended background.
                    if let Some(sub) = iter.next() {
                        match sub[0] {
                            2 => {
                                let r = iter.next().map_or(0, |p| p[0] as u8);
                                let g = iter.next().map_or(0, |p| p[0] as u8);
                                let b = iter.next().map_or(0, |p| p[0] as u8);
                                self.sgr.bg = Color::rgb(r, g, b);
                            }
                            5 => {
                                let n = iter.next().map_or(0, |p| p[0] as u8);
                                self.sgr.bg = Color::from_256(n);
                            }
                            _ => {}
                        }
                    }
                }
                49 => self.sgr.bg = Color::default_bg(),
                // Bright foreground (90-97)
                90..=97 => {
                    self.sgr.fg = ansi_bright_color(code - 90);
                }
                // Bright background (100-107)
                100..=107 => {
                    self.sgr.bg = ansi_bright_color(code - 100);
                }
                _ => {}
            }
        }
    }
}

/// Map ANSI basic color index (0-7) to RGB.
fn ansi_basic_color(idx: u16) -> Color {
    match idx {
        0 => Color::rgb(0, 0, 0),
        1 => Color::rgb(128, 0, 0),
        2 => Color::rgb(0, 128, 0),
        3 => Color::rgb(128, 128, 0),
        4 => Color::rgb(0, 0, 128),
        5 => Color::rgb(128, 0, 128),
        6 => Color::rgb(0, 128, 128),
        7 => Color::rgb(192, 192, 192),
        _ => Color::default_fg(),
    }
}

/// Map ANSI bright color index (0-7) to RGB.
fn ansi_bright_color(idx: u16) -> Color {
    match idx {
        0 => Color::rgb(128, 128, 128),
        1 => Color::rgb(255, 0, 0),
        2 => Color::rgb(0, 255, 0),
        3 => Color::rgb(255, 255, 0),
        4 => Color::rgb(0, 0, 255),
        5 => Color::rgb(255, 0, 255),
        6 => Color::rgb(0, 255, 255),
        7 => Color::rgb(255, 255, 255),
        _ => Color::default_fg(),
    }
}

impl Perform for VirtualTerminal {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // CR
            0x0D => self.cursor_col = 0,
            // LF
            0x0A => {
                self.cursor_row += 1;
                if self.cursor_row >= self.height {
                    // Scroll up: shift rows up by one, clear last row.
                    self.cells.remove(0);
                    self.cells.push(vec![Cell::default(); self.width]);
                    self.cursor_row = self.height - 1;
                }
            }
            // BS
            0x08 => {
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            // Tab
            0x09 => {
                let next_tab = (self.cursor_col / 8 + 1) * 8;
                self.cursor_col = next_tab.min(self.width - 1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let ps: Vec<Vec<u16>> = params.iter().map(|p| p.to_vec()).collect();

        match action {
            // Cursor Up
            'A' => {
                let n = ps.first().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            // Cursor Down
            'B' => {
                let n = ps.first().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_row = (self.cursor_row + n).min(self.height.saturating_sub(1));
            }
            // Cursor Forward
            'C' => {
                let n = ps.first().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = (self.cursor_col + n).min(self.width.saturating_sub(1));
            }
            // Cursor Back
            'D' => {
                let n = ps.first().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            // Cursor Position (CUP) / Horizontal Vertical Position (HVP)
            'H' | 'f' => {
                let row = ps.first().and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                let col = ps.get(1).and_then(|p| p.first().copied()).unwrap_or(1).max(1) as usize;
                self.cursor_row = (row - 1).min(self.height.saturating_sub(1));
                self.cursor_col = (col - 1).min(self.width.saturating_sub(1));
            }
            // Erase in Display
            'J' => {
                let mode = ps.first().and_then(|p| p.first().copied()).unwrap_or(0);
                self.erase_in_display(mode);
            }
            // Erase in Line
            'K' => {
                let mode = ps.first().and_then(|p| p.first().copied()).unwrap_or(0);
                self.erase_in_line(mode);
            }
            // SGR
            'm' => {
                self.apply_sgr(params);
            }
            // Save cursor position
            's' => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col));
            }
            // Restore cursor position
            'u' => {
                if let Some((r, c)) = self.saved_cursor {
                    self.cursor_row = r;
                    self.cursor_col = c;
                }
            }
            // DEC Private Mode Set/Reset (handled via intermediates '?')
            'h' | 'l' => {
                // The intermediates contain '?' for private modes.
                // vte passes private mode params with a leading flag in the param list.
                // We detect ?-prefixed modes by checking if the first subparam
                // is tagged (vte uses the leading byte in intermediates).
                // For vte 0.13, intermediates will contain b'?' for private modes.
                if _intermediates.contains(&b'?') {
                    let mode_val = ps.first().and_then(|p| p.first().copied()).unwrap_or(0);
                    let set = action == 'h';
                    match mode_val {
                        25 => {} // cursor visibility (no-op for vterm)
                        1049 => self.alt_screen = set,
                        2026 => {
                            if !set {
                                self.sync_end_seen = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_print() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed(b"Hello");
        assert_eq!(vt.row_text(0).trim(), "Hello");
    }

    #[test]
    fn cursor_movement() {
        let mut vt = VirtualTerminal::new(20, 5);
        // Move to row 2, col 5 (1-indexed)
        vt.feed(b"\x1b[2;5fTest");
        assert_eq!(&vt.row_text(1)[4..8], "Test");
    }

    #[test]
    fn sgr_24bit_color() {
        let mut vt = VirtualTerminal::new(10, 2);
        vt.feed(b"\x1b[38;2;255;128;0mX");
        let cell = &vt.cells[0][0];
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.fg, Color::rgb(255, 128, 0));
    }

    #[test]
    fn erase_in_display() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed(b"AAAAAAAAAA");
        vt.feed(b"\x1b[2J");
        assert_eq!(vt.row_text(0).trim(), "");
    }

    #[test]
    fn sync_end_detection() {
        let mut vt = VirtualTerminal::new(10, 3);
        assert!(!vt.sync_end_seen);
        vt.feed(b"\x1b[?2026h");
        assert!(!vt.sync_end_seen);
        vt.feed(b"\x1b[?2026l");
        assert!(vt.sync_end_seen);
    }

    #[test]
    fn cr_lf() {
        let mut vt = VirtualTerminal::new(20, 5);
        vt.feed(b"Line1\r\nLine2");
        assert_eq!(vt.row_text(0).trim(), "Line1");
        assert_eq!(vt.row_text(1).trim(), "Line2");
    }
}
