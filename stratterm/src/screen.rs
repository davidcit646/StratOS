use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    #[allow(dead_code)]
    RGB(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub underline: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: Color::Default,
            bg: Color::Default,
            bold: false,
            underline: false,
        }
    }
}

#[derive(Debug)]
pub struct ScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub rows: usize,
    pub cols: usize,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub current_fg: Color,
    pub current_bg: Color,
    pub current_bold: bool,
    pub current_underline: bool,
    pub scrollback: VecDeque<Vec<Cell>>,
    pub scrollback_max: usize,
    pub scrollback_offset: usize,
}

impl ScreenBuffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        let cells = vec![vec![Cell::default(); cols]; rows];
        ScreenBuffer {
            cells,
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            current_fg: Color::Default,
            current_bg: Color::Default,
            current_bold: false,
            current_underline: false,
            scrollback: VecDeque::new(),
            scrollback_max: 10_000,
            scrollback_offset: 0,
        }
    }

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        let mut new_cells = vec![vec![Cell::default(); new_cols]; new_rows];
        
        for (i, row) in new_cells.iter_mut().enumerate() {
            if i < self.cells.len() {
                for (j, cell) in row.iter_mut().enumerate() {
                    if j < self.cols {
                        *cell = self.cells[i][j];
                    }
                }
            }
        }
        
        self.cells = new_cells;
        self.rows = new_rows;
        self.cols = new_cols;
        self.scroll_top = 0;
        self.scroll_bottom = new_rows.saturating_sub(1);
        
        if self.cursor_row >= new_rows {
            self.cursor_row = new_rows.saturating_sub(1);
        }
        if self.cursor_col >= new_cols {
            self.cursor_col = new_cols.saturating_sub(1);
        }
        self.scrollback_offset = self
            .scrollback_offset
            .min(self.scrollback.len().saturating_add(self.rows).saturating_sub(self.rows));
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    pub fn clear_line(&mut self, row: usize) {
        if row < self.rows {
            for cell in &mut self.cells[row] {
                *cell = Cell::default();
            }
        }
    }

    pub fn put_char(&mut self, ch: char) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col].ch = ch;
            self.cells[self.cursor_row][self.cursor_col].fg = self.current_fg;
            self.cells[self.cursor_row][self.cursor_col].bg = self.current_bg;
            self.cells[self.cursor_row][self.cursor_col].bold = self.current_bold;
            self.cells[self.cursor_row][self.cursor_col].underline = self.current_underline;
            self.cursor_col += 1;
            if self.cursor_col >= self.cols {
                self.cursor_col = 0;
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    self.scroll_up(1);
                    self.cursor_row = self.rows.saturating_sub(1);
                }
            }
        }
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows.saturating_sub(1));
        self.cursor_col = col.min(self.cols.saturating_sub(1));
    }

    pub fn scroll_up(&mut self, count: usize) {
        let scroll_start = self.scroll_top;
        let scroll_end = self.scroll_bottom;
        
        if scroll_start >= scroll_end {
            return;
        }
        
        let scroll_range = scroll_end - scroll_start;
        let actual_count = count.min(scroll_range);
        
        for i in 0..actual_count {
            let source_row = scroll_start + i;
            if source_row < self.rows {
                self.push_scrollback_line(self.cells[source_row].clone());
            }
        }

        for i in scroll_start..=(scroll_end - actual_count) {
            if i + actual_count < self.rows {
                self.cells[i] = self.cells[i + actual_count].clone();
            }
        }
        
        for i in (scroll_end - actual_count + 1)..=scroll_end {
            if i < self.rows {
                for cell in &mut self.cells[i] {
                    *cell = Cell::default();
                }
            }
        }
    }

    fn push_scrollback_line(&mut self, line: Vec<Cell>) {
        self.scrollback.push_back(line);
        while self.scrollback.len() > self.scrollback_max {
            let _ = self.scrollback.pop_front();
        }
    }

    pub fn set_scrollback_max(&mut self, max_lines: usize) {
        self.scrollback_max = max_lines.max(1);
        while self.scrollback.len() > self.scrollback_max {
            let _ = self.scrollback.pop_front();
        }
        self.scrollback_offset = self.scrollback_offset.min(self.scrollback.len());
    }

    pub fn scrollback_page_up(&mut self, lines: usize) {
        let max_offset = self.scrollback.len();
        self.scrollback_offset = (self.scrollback_offset + lines).min(max_offset);
    }

    pub fn scrollback_page_down(&mut self, lines: usize) {
        self.scrollback_offset = self.scrollback_offset.saturating_sub(lines);
    }

    pub fn reset_scrollback(&mut self) {
        self.scrollback_offset = 0;
    }

    pub fn is_scrollback_active(&self) -> bool {
        self.scrollback_offset > 0
    }

    pub fn display_cell(&self, row: usize, col: usize) -> Cell {
        if row >= self.rows || col >= self.cols {
            return Cell::default();
        }

        let history_len = self.scrollback.len();
        let total_lines = history_len + self.rows;
        let window_start = total_lines.saturating_sub(self.rows + self.scrollback_offset);
        let global_row = window_start + row;

        if global_row < history_len {
            self.scrollback
                .get(global_row)
                .and_then(|line| line.get(col).copied())
                .unwrap_or_default()
        } else {
            let screen_row = global_row.saturating_sub(history_len);
            self.cells
                .get(screen_row)
                .and_then(|line| line.get(col).copied())
                .unwrap_or_default()
        }
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.scroll_top = top.min(self.rows.saturating_sub(1));
        self.scroll_bottom = bottom.min(self.rows.saturating_sub(1));
        if self.scroll_bottom <= self.scroll_top {
            self.scroll_bottom = self.rows.saturating_sub(1);
        }
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.current_fg = fg;
        self.current_bg = bg;
    }

    pub fn set_foreground(&mut self, fg: Color) {
        self.current_fg = fg;
    }

    pub fn set_background(&mut self, bg: Color) {
        self.current_bg = bg;
    }

    pub fn set_bold(&mut self, bold: bool) {
        self.current_bold = bold;
    }

    pub fn set_underline(&mut self, underline: bool) {
        self.current_underline = underline;
    }
}
