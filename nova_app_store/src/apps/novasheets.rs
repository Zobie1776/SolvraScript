use crate::app::{
    AppCapability, AppCategory, AppId, AppMetadata, AppPackage, UiComponent, UiComponentKind,
};
use crate::sandbox::{SandboxPermission, SandboxPolicy};
use semver::Version;
use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// Return catalog metadata for NovaSheets.
pub fn metadata() -> AppMetadata {
    let sandbox = SandboxPolicy::new()
        .allow_permission(SandboxPermission::FileRead)
        .allow_permission(SandboxPermission::FileWrite)
        .allow_storage_root("~/Documents/NovaSheets");

    let package = AppPackage::new(Version::new(1, 0, 0))
        .with_sandbox(sandbox)
        .with_capability(
            AppCapability::new(
                "novasheets.grid",
                "High-performance spreadsheet engine with formula evaluation",
            )
            .with_tag("spreadsheet")
            .with_tag("analysis"),
        )
        .with_ui_component(
            UiComponent::new(
                "novasheets-grid",
                UiComponentKind::IdeView,
                "Interactive spreadsheet grid with formula bar",
            )
            .with_entry_point("novasheets::grid"),
        );

    AppMetadata::new(
        AppId::new("dev.nova.sheets").expect("valid id"),
        "NovaSheets",
        "Flexible spreadsheet with formulas and chart-ready data",
        "NovaSheets provides an opinionated yet powerful spreadsheet workflow with fast recalculation, CSV import/export, and integration hooks for NovaScript automation.",
        AppCategory::Productivity,
        "Nova Labs",
        package,
    )
    .with_tag("data")
    .with_screenshot("screenshots/novasheets.png")
}

/// Errors that can occur while manipulating a sheet.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SheetError {
    #[error("invalid cell reference '{0}'")]
    InvalidReference(String),
    #[error("circular reference detected at '{0}'")]
    CircularReference(String),
    #[error("formula parse error: {0}")]
    Parse(String),
}

/// Represents the contents of a spreadsheet cell.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CellContent {
    #[default]
    Empty,
    Number(f64),
    Text(String),
    Formula(String),
}

/// Runtime value of a cell after formula evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Empty,
    Number(f64),
    Text(String),
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellValue::Empty => Ok(()),
            CellValue::Number(n) => write!(f, "{}", n),
            CellValue::Text(t) => write!(f, "{}", t),
        }
    }
}

/// Coordinate pair identifying a cell in the sheet.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellCoordinate {
    pub column: usize,
    pub row: usize,
}

impl fmt::Display for CellCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut col = self.column;
        let mut label = String::new();
        loop {
            let rem = col % 26;
            label.push((b'A' + rem as u8) as char);
            col /= 26;
            if col == 0 {
                break;
            }
            col -= 1;
        }
        write!(
            f,
            "{}{}",
            label.chars().rev().collect::<String>(),
            self.row + 1
        )
    }
}

impl CellCoordinate {
    pub fn parse(reference: &str) -> Result<Self, SheetError> {
        let mut chars = reference.chars();
        let mut col_label = String::new();
        while let Some(ch) = chars.next() {
            if ch.is_ascii_alphabetic() {
                col_label.push(ch.to_ascii_uppercase());
            } else {
                let mut row_label = String::new();
                row_label.push(ch);
                row_label.extend(chars);
                if row_label.is_empty() || col_label.is_empty() {
                    return Err(SheetError::InvalidReference(reference.into()));
                }
                let row: usize = row_label
                    .parse::<usize>()
                    .map_err(|_| SheetError::InvalidReference(reference.into()))?;
                return Ok(Self {
                    column: Self::column_index(&col_label),
                    row: row - 1,
                });
            }
        }
        Err(SheetError::InvalidReference(reference.into()))
    }

    fn column_index(label: &str) -> usize {
        let mut value = 0usize;
        for (idx, ch) in label.chars().rev().enumerate() {
            let digit = (ch as u8 - b'A') as usize + 1;
            value += digit * 26usize.pow(idx as u32);
        }
        value - 1
    }
}

/// Spreadsheet model supporting formulas, references, and CSV import/export.
#[derive(Debug, Default, Clone)]
pub struct Sheet {
    cells: HashMap<CellCoordinate, CellContent>,
}

impl Sheet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the raw content of a cell.
    pub fn set_cell(
        &mut self,
        reference: &str,
        value: impl Into<String>,
    ) -> Result<(), SheetError> {
        let coord = CellCoordinate::parse(reference)?;
        let value = value.into();
        let content = if let Some(formula) = value.strip_prefix('=') {
            CellContent::Formula(formula.trim().to_string())
        } else if let Ok(number) = value.parse::<f64>() {
            CellContent::Number(number)
        } else if value.trim().is_empty() {
            CellContent::Empty
        } else {
            CellContent::Text(value)
        };
        if content == CellContent::Empty {
            self.cells.remove(&coord);
        } else {
            self.cells.insert(coord, content);
        }
        Ok(())
    }

    /// Retrieve the computed value for a cell.
    pub fn value(&self, reference: &str) -> Result<CellValue, SheetError> {
        let coord = CellCoordinate::parse(reference)?;
        let mut visited = BTreeSet::new();
        self.evaluate_cell(&coord, &mut visited)
    }

    /// Export the sheet as CSV.
    pub fn to_csv(&self) -> String {
        let mut rows: Vec<usize> = self.cells.keys().map(|coord| coord.row).collect();
        rows.sort_unstable();
        rows.dedup();
        let mut csv = String::new();
        for row in rows {
            let mut columns: Vec<usize> = self
                .cells
                .keys()
                .filter(|coord| coord.row == row)
                .map(|coord| coord.column)
                .collect();
            columns.sort_unstable();
            let mut cells = Vec::new();
            for column in columns {
                let coord = CellCoordinate { column, row };
                if let Some(content) = self.cells.get(&coord) {
                    let text = match content {
                        CellContent::Empty => String::new(),
                        CellContent::Number(n) => n.to_string(),
                        CellContent::Text(t) => t.clone(),
                        CellContent::Formula(f) => format!("={}", f),
                    };
                    cells.push(text);
                }
            }
            csv.push_str(&cells.join(","));
            csv.push('\n');
        }
        csv
    }

    /// Import CSV data into the sheet.
    pub fn import_csv(&mut self, csv: &str) {
        self.cells.clear();
        for (row_idx, line) in csv.lines().enumerate() {
            for (col_idx, value) in line.split(',').enumerate() {
                if value.is_empty() {
                    continue;
                }
                let coord = CellCoordinate {
                    column: col_idx,
                    row: row_idx,
                };
                let content = if let Some(stripped) = value.strip_prefix('=') {
                    CellContent::Formula(stripped.trim().to_string())
                } else if let Ok(number) = value.parse::<f64>() {
                    CellContent::Number(number)
                } else {
                    CellContent::Text(value.to_string())
                };
                self.cells.insert(coord, content);
            }
        }
    }

    fn evaluate_cell(
        &self,
        coord: &CellCoordinate,
        visited: &mut BTreeSet<CellCoordinate>,
    ) -> Result<CellValue, SheetError> {
        if !visited.insert(coord.clone()) {
            return Err(SheetError::CircularReference(coord.to_string()));
        }
        let content = self.cells.get(coord).cloned().unwrap_or_default();
        let value = match content {
            CellContent::Empty => CellValue::Empty,
            CellContent::Number(n) => CellValue::Number(n),
            CellContent::Text(text) => CellValue::Text(text),
            CellContent::Formula(formula) => {
                let result = self.evaluate_formula(&formula, visited)?;
                CellValue::Number(result)
            }
        };
        visited.remove(coord);
        Ok(value)
    }

    fn evaluate_formula(
        &self,
        formula: &str,
        visited: &mut BTreeSet<CellCoordinate>,
    ) -> Result<f64, SheetError> {
        let mut parser = FormulaParser::new(self, visited, formula);
        let value = parser.parse_expression()?;
        parser.consume_whitespace();
        if !parser.is_eof() {
            return Err(SheetError::Parse(format!(
                "unexpected token near '{}...'",
                parser.remaining()
            )));
        }
        Ok(value)
    }
}

/// Simple recursive descent parser for sheet formulas.
struct FormulaParser<'a> {
    sheet: &'a Sheet,
    visited: &'a mut BTreeSet<CellCoordinate>,
    input: &'a str,
    pos: usize,
}

impl<'a> FormulaParser<'a> {
    fn new(sheet: &'a Sheet, visited: &'a mut BTreeSet<CellCoordinate>, input: &'a str) -> Self {
        Self {
            sheet,
            visited,
            input,
            pos: 0,
        }
    }

    fn parse_expression(&mut self) -> Result<f64, SheetError> {
        let mut value = self.parse_term()?;
        loop {
            self.consume_whitespace();
            if self.consume_char('+') {
                value += self.parse_term()?;
            } else if self.consume_char('-') {
                value -= self.parse_term()?;
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, SheetError> {
        let mut value = self.parse_factor()?;
        loop {
            self.consume_whitespace();
            if self.consume_char('*') {
                value *= self.parse_factor()?;
            } else if self.consume_char('/') {
                value /= self.parse_factor()?;
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, SheetError> {
        self.consume_whitespace();
        if self.consume_char('(') {
            let value = self.parse_expression()?;
            self.consume_whitespace();
            if !self.consume_char(')') {
                return Err(SheetError::Parse("expected ')'".into()));
            }
            Ok(value)
        } else if self.peek_char() == Some('+') {
            self.consume_char('+');
            self.parse_factor()
        } else if self.peek_char() == Some('-') {
            self.consume_char('-');
            Ok(-self.parse_factor()?)
        } else if let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.parse_number()
            } else if ch.is_ascii_alphabetic() {
                self.parse_identifier_or_reference()
            } else {
                Err(SheetError::Parse(format!("unexpected character '{}'", ch)))
            }
        } else {
            Err(SheetError::Parse("unexpected end of formula".into()))
        }
    }

    fn parse_identifier_or_reference(&mut self) -> Result<f64, SheetError> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphabetic() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let name = &self.input[start..self.pos];
        if self.peek_char() == Some('(') {
            self.consume_char('(');
            let value = self.parse_function(name)?;
            self.consume_whitespace();
            if !self.consume_char(')') {
                return Err(SheetError::Parse("expected ')' after function call".into()));
            }
            Ok(value)
        } else {
            self.parse_reference(name)
        }
    }

    fn parse_function(&mut self, name: &str) -> Result<f64, SheetError> {
        let mut values = Vec::new();
        loop {
            self.consume_whitespace();
            if self.peek_char() == Some(')') {
                break;
            }
            values.extend(self.parse_function_values()?);
            self.consume_whitespace();
            if !self.consume_char(',') {
                break;
            }
        }
        match name.to_ascii_uppercase().as_str() {
            "SUM" => Ok(values.iter().copied().sum()),
            "AVG" | "AVERAGE" => {
                if values.is_empty() {
                    Ok(0.0)
                } else {
                    let sum: f64 = values.iter().copied().sum();
                    Ok(sum / values.len() as f64)
                }
            }
            _ => Err(SheetError::Parse(format!("unknown function {}", name))),
        }
    }

    fn parse_function_values(&mut self) -> Result<Vec<f64>, SheetError> {
        self.consume_whitespace();
        let save = self.pos;
        if let Some((left, right)) = self.try_parse_range()? {
            return self.collect_range_values(&left, &right);
        }
        self.pos = save;
        Ok(vec![self.parse_expression()?])
    }

    fn parse_reference(&mut self, prefix: &str) -> Result<f64, SheetError> {
        let label = format!("{}{}", prefix, self.parse_digits()?);
        let coord = CellCoordinate::parse(&label)?;
        match self.sheet.evaluate_cell(&coord, self.visited)? {
            CellValue::Number(n) => Ok(n),
            CellValue::Text(_) => Err(SheetError::Parse(format!(
                "cell {} contains non-numeric data",
                coord
            ))),
            CellValue::Empty => Ok(0.0),
        }
    }

    fn parse_cell_label(&mut self) -> Result<String, SheetError> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if start == self.pos {
            Err(SheetError::Parse("expected cell reference".into()))
        } else {
            Ok(self.input[start..self.pos].to_string())
        }
    }

    fn collect_range_values(
        &mut self,
        left: &CellCoordinate,
        right: &CellCoordinate,
    ) -> Result<Vec<f64>, SheetError> {
        let (col_start, col_end) = if left.column <= right.column {
            (left.column, right.column)
        } else {
            (right.column, left.column)
        };
        let (row_start, row_end) = if left.row <= right.row {
            (left.row, right.row)
        } else {
            (right.row, left.row)
        };
        let mut values = Vec::new();
        for col in col_start..=col_end {
            for row in row_start..=row_end {
                let coord = CellCoordinate { column: col, row };
                if let CellValue::Number(value) = self.sheet.evaluate_cell(&coord, self.visited)? {
                    values.push(value);
                }
            }
        }
        Ok(values)
    }

    fn try_parse_range(&mut self) -> Result<Option<(CellCoordinate, CellCoordinate)>, SheetError> {
        let save = self.pos;
        match self.parse_cell_label() {
            Ok(left_label) => {
                if !self.consume_char(':') {
                    self.pos = save;
                    return Ok(None);
                }
                let right_label = match self.parse_cell_label() {
                    Ok(label) => label,
                    Err(err) => {
                        self.pos = save;
                        return Err(err);
                    }
                };
                let left = CellCoordinate::parse(&left_label)?;
                let right = CellCoordinate::parse(&right_label)?;
                Ok(Some((left, right)))
            }
            Err(_) => {
                self.pos = save;
                Ok(None)
            }
        }
    }

    fn parse_number(&mut self) -> Result<f64, SheetError> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|_| SheetError::Parse("invalid number".into()))
    }

    fn parse_digits(&mut self) -> Result<String, SheetError> {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if start == self.pos {
            Err(SheetError::Parse("expected digits".into()))
        } else {
            Ok(self.input[start..self.pos].to_string())
        }
    }

    fn consume_char(&mut self, ch: char) -> bool {
        if self.peek_char() == Some(ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn consume_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn remaining(&self) -> &str {
        &self.input[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_basic_formulas() {
        let mut sheet = Sheet::new();
        sheet.set_cell("A1", "4").unwrap();
        sheet.set_cell("A2", "6").unwrap();
        sheet.set_cell("A3", "=A1 + A2 * 2").unwrap();
        assert_eq!(sheet.value("A3").unwrap(), CellValue::Number(16.0));
    }

    #[test]
    fn evaluates_sum_ranges() {
        let mut sheet = Sheet::new();
        sheet.set_cell("A1", "1").unwrap();
        sheet.set_cell("A2", "2").unwrap();
        sheet.set_cell("A3", "3").unwrap();
        sheet.set_cell("A4", "=SUM(A1:A3)").unwrap();
        assert_eq!(sheet.value("A4").unwrap(), CellValue::Number(6.0));
    }

    #[test]
    fn detects_circular_references() {
        let mut sheet = Sheet::new();
        sheet.set_cell("A1", "=A2").unwrap();
        sheet.set_cell("A2", "=A1").unwrap();
        assert!(matches!(
            sheet.value("A1"),
            Err(SheetError::CircularReference(_))
        ));
    }
}
