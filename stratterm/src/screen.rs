#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
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

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.scroll_top = top.min(self.rows.saturating_sub(1));
        self.scroll_bottom = bottom.min(self.rows.saturating_sub(1));
        if self.scroll_bottom <= self.scroll_top {
            self.scroll_bottom = self.rows.saturating_sub(1);
        }
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col].fg = fg;
            self.cells[self.cursor_row][self.cursor_col].bg = bg;
        }
    }

    pub fn set_bold(&mut self, bold: bool) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col].bold = bold;
        }
    }

    pub fn set_underline(&mut self, underline: bool) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col].underline = underline;
        }
    }
}
