use std::cmp::max;

use unicode_width::UnicodeWidthChar;

use ui_model::{Attrs, UiModel};

pub struct ModelLayout {
    pub model: UiModel,
    rows_filled: usize,
    cols_filled: usize,
    lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>,
}

impl ModelLayout {
    const ROWS_STEP: usize = 10;

    pub fn new(columns: u64) -> Self {
        ModelLayout {
            model: UiModel::new(ModelLayout::ROWS_STEP as u64, columns),
            rows_filled: 0,
            cols_filled: 0,
            lines: Vec::new(),
        }
    }

    pub fn layout_append(&mut self, mut lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>) {
        let rows_filled = self.rows_filled;
        let take_from = self.lines.len();

        self.lines.append(&mut lines);

        self.layout_replace(rows_filled, take_from);
    }

    pub fn layout(&mut self, lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>) {
        self.lines = lines;
        self.layout_replace(0, 0);
    }

    pub fn set_cursor(&mut self, col: usize) {
        let row = if self.rows_filled > 0 {
            self.rows_filled - 1
        } else {
            0
        };

        self.model.set_cursor(row, col);
    }

    pub fn size(&self) -> (usize, usize) {
        (
            max(self.cols_filled, self.model.get_cursor().1 + 1),
            self.rows_filled,
        )
    }

    fn check_model_size(&mut self, rows: usize) {
        if rows > self.model.rows {
            let model_cols = self.model.columns;
            let model_rows = ((rows / (ModelLayout::ROWS_STEP + 1)) + 1) * ModelLayout::ROWS_STEP;
            let (cur_row, cur_col) = self.model.get_cursor();

            let mut model = UiModel::new(model_rows as u64, model_cols as u64);
            self.model.copy_rows(&mut model, self.rows_filled - 1);
            model.set_cursor(cur_row, cur_col);
            self.model = model;
        }
    }

    pub fn insert_char(&mut self, c: &str, shift: bool) {
        if c.is_empty() {
            return;
        }

        let ch = c.chars().next().unwrap();
        let (row, col) = self.model.get_cursor();

        if shift {
            self.insert_into_lines(ch);
            self.layout_replace(0, 0);
        } else {
            self.model.put(ch, false, None);
        }

        self.model.set_cursor(row, col);
    }

    fn insert_into_lines(&mut self, ch: char) {
        let line = &mut self.lines[0];

        let cur_col = self.model.cur_col;

        let mut col_idx = 0;
        for &mut (_, ref mut chars) in line {
            if cur_col < col_idx + chars.len() {
                let col_sub_idx = cur_col - col_idx;
                chars.insert(col_sub_idx, ch);
            } else {
                col_idx += chars.len();
            }
        }
    }

    /// Wrap all lines into model
    ///
    /// returns actual width
    fn layout_replace(&mut self, row_offset: usize, take_from: usize) {
        let rows = ModelLayout::count_lines(&self.lines[take_from..], self.model.columns);

        self.check_model_size(rows + row_offset);
        self.rows_filled = rows + row_offset;

        let lines = &self.lines[take_from..];

        let mut max_col_idx = 0;
        let mut col_idx = 0;
        let mut row_idx = row_offset;
        for content in lines {
            for &(ref attr, ref ch_list) in content {
                for ch in ch_list {
                    let ch_width = ch.width().unwrap_or(1);

                    if col_idx + ch_width > self.model.columns {
                        col_idx = 0;
                        row_idx += 1;
                    }

                    self.model.set_cursor(row_idx, col_idx as usize);
                    self.model.put(*ch, false, attr.as_ref());
                    if ch_width > 1 {
                        self.model.put(' ', true, attr.as_ref());
                    }

                    if max_col_idx < col_idx {
                        max_col_idx = col_idx + ch_width - 1;
                    }

                    col_idx += ch_width;
                }

                if col_idx < self.model.columns {
                    self.model.model[row_idx].clear(col_idx, self.model.columns - 1);
                }
            }
            row_idx += 1;
        }

        if self.rows_filled == 1 {
            self.cols_filled = max_col_idx + 1;
        } else {
            self.cols_filled = max(self.cols_filled, max_col_idx + 1);
        }
    }

    fn count_lines(lines: &[Vec<(Option<Attrs>, Vec<char>)>], max_columns: usize) -> usize {
        let mut row_count = 0;

        for line in lines {
            let len: usize = line.iter().map(|c| c.1.len()).sum();
            row_count += len / (max_columns + 1) + 1;
        }

        row_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines() {
        let lines = vec![vec![(None, vec!['a'; 5])]];

        let rows = ModelLayout::count_lines(&lines, 4);
        assert_eq!(2, rows);
    }

    #[test]
    fn test_resize() {
        let lines = vec![vec![(None, vec!['a'; 5])]; ModelLayout::ROWS_STEP];
        let mut model = ModelLayout::new(5);

        model.layout(lines.clone());
        let (cols, rows) = model.size();
        assert_eq!(5, cols);
        assert_eq!(ModelLayout::ROWS_STEP, rows);

        model.layout_append(lines);
        let (cols, rows) = model.size();
        assert_eq!(5, cols);
        assert_eq!(ModelLayout::ROWS_STEP * 2, rows);
        assert_eq!(ModelLayout::ROWS_STEP * 2, model.model.rows);
    }

    #[test]
    fn test_cols_filled() {
        let lines = vec![vec![(None, vec!['a'; 3])]; 1];
        let mut model = ModelLayout::new(5);

        model.layout(lines);
        let (cols, _) = model.size();
        assert_eq!(4, cols); // size is 3 and 4 - is with cursor position

        let lines = vec![vec![(None, vec!['a'; 2])]; 1];

        model.layout_append(lines);
        let (cols, _) = model.size();
        assert_eq!(3, cols);
    }

    #[test]
    fn test_insert_shift() {
        let lines = vec![vec![(None, vec!['a'; 3])]; 1];
        let mut model = ModelLayout::new(5);
        model.layout(lines);
        model.set_cursor(1);

        model.insert_char("b", true);

        let (cols, _) = model.size();
        assert_eq!(4, cols);
        assert_eq!('b', model.model.model()[0].line[1].ch);
    }

    #[test]
    fn test_insert_no_shift() {
        let lines = vec![vec![(None, vec!['a'; 3])]; 1];
        let mut model = ModelLayout::new(5);
        model.layout(lines);
        model.set_cursor(1);

        model.insert_char("b", false);

        let (cols, _) = model.size();
        assert_eq!(3, cols);
        assert_eq!('b', model.model.model()[0].line[1].ch);
    }

    #[test]
    fn test_double_width() {
        let lines = vec![vec![(None, vec!['あ'; 3])]; 1];
        let mut model = ModelLayout::new(7);
        model.layout(lines);
        model.set_cursor(1);

        let (cols, rows) = model.size();
        assert_eq!(1, rows);
        assert_eq!(6, cols);
    }
}
