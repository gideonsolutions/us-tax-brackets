//! Compute U.S. federal income tax from IRS tax tables and computation worksheets.
//!
//! This crate provides federal income tax computation based on official IRS data
//! scraped from the [Form 1040 instructions](https://www.irs.gov/instructions/i1040gi).
//!
//! # Usage
//!
//! ```
//! use us_tax_brackets::{FilingStatus, TaxYear, compute_tax};
//!
//! // Compute tax for a single filer with $75,000 taxable income in 2025
//! let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 75_000).unwrap();
//! assert_eq!(tax, 11_420);
//!
//! // Compute tax for married filing jointly with $200,000 taxable income
//! let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
//! assert_eq!(tax, 33_828);
//! ```

use std::fmt;

/// Tax year supported by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxYear {
    Y2025,
}

impl fmt::Display for TaxYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaxYear::Y2025 => write!(f, "2025"),
        }
    }
}

/// Filing status for federal income tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilingStatus {
    Single,
    MarriedFilingJointly,
    MarriedFilingSeparately,
    HeadOfHousehold,
}

impl fmt::Display for FilingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilingStatus::Single => write!(f, "Single"),
            FilingStatus::MarriedFilingJointly => write!(f, "Married Filing Jointly"),
            FilingStatus::MarriedFilingSeparately => write!(f, "Married Filing Separately"),
            FilingStatus::HeadOfHousehold => write!(f, "Head of Household"),
        }
    }
}

/// Error type for tax computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaxError {
    /// Taxable income is negative.
    NegativeIncome,
    /// No matching bracket found for the given income.
    NoBracketFound,
}

impl fmt::Display for TaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaxError::NegativeIncome => write!(f, "taxable income cannot be negative"),
            TaxError::NoBracketFound => write!(f, "no matching tax bracket found"),
        }
    }
}

impl std::error::Error for TaxError {}

/// A row from the IRS Tax Table (income < $100,000).
/// Tax amounts are pre-computed by the IRS for $50 income ranges.
struct TaxTableRow {
    income_min: i64,
    income_max: i64,
    single: i64,
    married_filing_jointly: i64,
    married_filing_separately: i64,
    head_of_household: i64,
}

/// A bracket from the Tax Computation Worksheet (income >= $100,000).
/// Tax = income * rate - subtraction_amount
struct WorksheetBracket {
    income_min: i64,
    /// `None` means no upper bound.
    income_max: Option<i64>,
    rate: f64,
    subtraction_amount: f64,
}

// Embed CSV data at compile time.
const TAX_TABLE_CSV_2025: &str = include_str!("../data/2025/tax_table.csv");
const WORKSHEET_CSV_2025: &str = include_str!("../data/2025/tax_computation_worksheet.csv");

fn parse_tax_table(csv: &str) -> Vec<TaxTableRow> {
    csv.lines()
        .skip(1) // header
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 6 {
                return None;
            }
            Some(TaxTableRow {
                income_min: cols[0].parse().ok()?,
                income_max: cols[1].parse().ok()?,
                single: cols[2].parse().ok()?,
                married_filing_jointly: cols[3].parse().ok()?,
                married_filing_separately: cols[4].parse().ok()?,
                head_of_household: cols[5].parse().ok()?,
            })
        })
        .collect()
}

fn parse_worksheet(csv: &str, status_key: &str) -> Vec<WorksheetBracket> {
    csv.lines()
        .skip(1) // header
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 5 {
                return None;
            }
            if cols[0] != status_key {
                return None;
            }
            Some(WorksheetBracket {
                income_min: cols[1].parse().ok()?,
                income_max: if cols[2].is_empty() {
                    None
                } else {
                    Some(cols[2].parse().ok()?)
                },
                rate: cols[3].parse().ok()?,
                subtraction_amount: cols[4].parse().ok()?,
            })
        })
        .collect()
}

fn filing_status_csv_key(status: FilingStatus) -> &'static str {
    match status {
        FilingStatus::Single => "single",
        FilingStatus::MarriedFilingJointly => "married_filing_jointly",
        FilingStatus::MarriedFilingSeparately => "married_filing_separately",
        FilingStatus::HeadOfHousehold => "head_of_household",
    }
}

/// Compute federal income tax for a given tax year, filing status, and taxable income.
///
/// The `taxable_income` parameter is typically line 15 of Form 1040 (often derived
/// from MAGI minus deductions). The value should be in whole dollars.
///
/// Returns the computed tax in whole dollars (rounded to nearest dollar as the IRS
/// tax table does).
pub fn compute_tax(
    year: TaxYear,
    status: FilingStatus,
    taxable_income: i64,
) -> Result<i64, TaxError> {
    if taxable_income < 0 {
        return Err(TaxError::NegativeIncome);
    }
    if taxable_income == 0 {
        return Ok(0);
    }

    let (table_csv, worksheet_csv) = match year {
        TaxYear::Y2025 => (TAX_TABLE_CSV_2025, WORKSHEET_CSV_2025),
    };

    if taxable_income < 100_000 {
        // Use the Tax Table: find the row where income_min <= taxable_income < income_max
        let table = parse_tax_table(table_csv);

        // Binary search: rows are sorted by income_min in $50 increments
        let idx = table
            .binary_search_by(|row| {
                if taxable_income < row.income_min {
                    std::cmp::Ordering::Greater
                } else if taxable_income >= row.income_max {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .map_err(|_| TaxError::NoBracketFound)?;

        let row = &table[idx];
        Ok(match status {
            FilingStatus::Single => row.single,
            FilingStatus::MarriedFilingJointly => row.married_filing_jointly,
            FilingStatus::MarriedFilingSeparately => row.married_filing_separately,
            FilingStatus::HeadOfHousehold => row.head_of_household,
        })
    } else {
        // Use the Tax Computation Worksheet: tax = income * rate - subtraction_amount
        let key = filing_status_csv_key(status);
        let brackets = parse_worksheet(worksheet_csv, key);

        for bracket in &brackets {
            let in_range = match bracket.income_max {
                Some(max) => taxable_income >= bracket.income_min && taxable_income <= max,
                None => taxable_income > bracket.income_min,
            };
            if in_range {
                let tax = (taxable_income as f64) * bracket.rate - bracket.subtraction_amount;
                return Ok(tax.round() as i64);
            }
        }

        Err(TaxError::NoBracketFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_income() {
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 0).unwrap();
        assert_eq!(tax, 0);
    }

    #[test]
    fn test_negative_income() {
        let result = compute_tax(TaxYear::Y2025, FilingStatus::Single, -1);
        assert_eq!(result, Err(TaxError::NegativeIncome));
    }

    #[test]
    fn test_low_income_single() {
        // $10 taxable income -> $1 tax (from tax table: 5-15 range)
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 10).unwrap();
        assert_eq!(tax, 1);
    }

    #[test]
    fn test_tax_table_single_50k() {
        // $50,000 single: from IRS tax table
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 50_000).unwrap();
        assert_eq!(tax, 5_920);
    }

    #[test]
    fn test_tax_table_married_jointly_75k() {
        // $75,000 married filing jointly: from IRS tax table
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 75_000).unwrap();
        assert_eq!(tax, 8_526);
    }

    #[test]
    fn test_worksheet_single_150k() {
        // $150,000 single: 150000 * 0.24 - 7153 = 36000 - 7153 = 28847
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 150_000).unwrap();
        assert_eq!(tax, 28_847);
    }

    #[test]
    fn test_worksheet_married_jointly_200k() {
        // $200,000 MFJ: 200000 * 0.22 - 10172 = 44000 - 10172 = 33828
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
        assert_eq!(tax, 33_828);
    }

    #[test]
    fn test_worksheet_single_1m() {
        // $1,000,000 single: 1000000 * 0.37 - 42979.75 = 370000 - 42979.75 = 327020.25 -> 327020
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 1_000_000).unwrap();
        assert_eq!(tax, 327_020);
    }

    #[test]
    fn test_boundary_100k_single() {
        // $100,000 is the first row of the worksheet: "At least $100,000 but not over $103,350"
        // 100000 * 0.22 - 5086 = 22000 - 5086 = 16914
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 100_000).unwrap();
        assert_eq!(tax, 16_914);
    }

    #[test]
    fn test_boundary_99999_single() {
        // $99,999 should use the tax table (last row before $100k)
        // Row: 99950-100000 -> single = 16909
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 99_999).unwrap();
        assert_eq!(tax, 16_909);
    }

    #[test]
    fn test_all_filing_statuses_at_200k() {
        let s = compute_tax(TaxYear::Y2025, FilingStatus::Single, 200_000).unwrap();
        let mfj = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
        let mfs =
            compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingSeparately, 200_000).unwrap();
        let hoh = compute_tax(TaxYear::Y2025, FilingStatus::HeadOfHousehold, 200_000).unwrap();

        // MFJ should have the lowest tax at the same income
        assert!(mfj < s);
        assert!(mfj < mfs);
        assert!(mfj < hoh);
        // Single: 200000 * 0.32 - 22937 = 41063
        assert_eq!(s, 41_063);
        // MFJ: 200000 * 0.22 - 10172 = 33828
        assert_eq!(mfj, 33_828);
        // MFS same brackets as single at this range
        assert_eq!(mfs, 41_063);
        // HoH: 200000 * 0.32 - 24676 = 39324
        assert_eq!(hoh, 39_324);
    }

    #[test]
    fn test_head_of_household_worksheet() {
        // $300,000 HoH: 300000 * 0.35 - 32191 = 105000 - 32191 = 72809
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::HeadOfHousehold, 300_000).unwrap();
        assert_eq!(tax, 72_809);
    }
}
