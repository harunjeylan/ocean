use crate::ocean_fs::parser::xlsx::{CellRef, RangeRef};

#[test]
fn test_cell_ref_parsing() {
    assert_eq!(CellRef::parse("A1").unwrap(), CellRef { col: 0, row: 0 });
    assert_eq!(CellRef::parse("Z1").unwrap(), CellRef { col: 25, row: 0 });
    assert_eq!(CellRef::parse("AA1").unwrap(), CellRef { col: 26, row: 0 });
    assert_eq!(CellRef::parse("A10").unwrap(), CellRef { col: 0, row: 9 });
}

#[test]
fn test_cell_ref_notation() {
    assert_eq!(CellRef { col: 0, row: 0 }.to_notation(), "A1");
    assert_eq!(CellRef { col: 25, row: 0 }.to_notation(), "Z1");
    assert_eq!(CellRef { col: 26, row: 0 }.to_notation(), "AA1");
    assert_eq!(CellRef { col: 0, row: 9 }.to_notation(), "A10");
}

#[test]
fn test_range_parsing() {
    let range = RangeRef::parse("A1:B2").unwrap();
    assert_eq!(range.cells().len(), 4);
}
