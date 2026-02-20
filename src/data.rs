//! Embedded IRS tax data and CSV parsing.
//!
//! Tax data is scraped from the IRS Form 1040 instructions and stored as CSV
//! files in the repository's `data/<year>/` directories. The CSV files are
//! embedded into the binary at compile time using [`include_str!`], so no
//! runtime file I/O is needed.

use crate::types::FilingStatus;

// ---------------------------------------------------------------------------
// Embedded CSV data
// ---------------------------------------------------------------------------

/// Tax Table CSV for tax year 2023 (income $0–$99,999).
const TAX_TABLE_CSV_2023: &str = include_str!("../data/2023/tax_table.csv");

/// Tax Computation Worksheet CSV for tax year 2023 (income $100,000+).
const WORKSHEET_CSV_2023: &str = include_str!("../data/2023/tax_computation_worksheet.csv");

/// Tax Table CSV for tax year 2024 (income $0–$99,999).
const TAX_TABLE_CSV_2024: &str = include_str!("../data/2024/tax_table.csv");

/// Tax Computation Worksheet CSV for tax year 2024 (income $100,000+).
const WORKSHEET_CSV_2024: &str = include_str!("../data/2024/tax_computation_worksheet.csv");

/// Tax Table CSV for tax year 2025 (income $0–$99,999).
const TAX_TABLE_CSV_2025: &str = include_str!("../data/2025/tax_table.csv");

/// Tax Computation Worksheet CSV for tax year 2025 (income $100,000+).
const WORKSHEET_CSV_2025: &str = include_str!("../data/2025/tax_computation_worksheet.csv");

/// Return the embedded (Tax Table CSV, Worksheet CSV) for the given tax year.
pub(crate) fn csv_for_year(year: crate::types::TaxYear) -> (&'static str, &'static str) {
    use crate::types::TaxYear;
    match year {
        TaxYear::Y2023 => (TAX_TABLE_CSV_2023, WORKSHEET_CSV_2023),
        TaxYear::Y2024 => (TAX_TABLE_CSV_2024, WORKSHEET_CSV_2024),
        TaxYear::Y2025 => (TAX_TABLE_CSV_2025, WORKSHEET_CSV_2025),
    }
}

// ---------------------------------------------------------------------------
// Internal data structures
// ---------------------------------------------------------------------------

/// A single row from the IRS Tax Table.
///
/// Each row covers a $50 income range and contains the pre-computed tax amount
/// for every filing status.
pub(crate) struct TaxTableRow {
    /// Lower bound of the income range (inclusive).
    pub income_min: i64,
    /// Upper bound of the income range (exclusive).
    pub income_max: i64,
    pub single: i64,
    pub married_filing_jointly: i64,
    pub married_filing_separately: i64,
    pub head_of_household: i64,
}

/// A single bracket from the Tax Computation Worksheet.
///
/// For incomes of $100,000 or more, the IRS provides a formula:
///
/// ```text
/// tax = taxable_income × rate − subtraction_amount
/// ```
pub(crate) struct WorksheetBracket {
    /// Lower bound of the bracket (inclusive for the first bracket, exclusive
    /// for "Over $X" brackets).
    pub income_min: i64,
    /// Upper bound of the bracket (inclusive), or [`None`] for the highest
    /// (unbounded) bracket.
    pub income_max: Option<i64>,
    /// Marginal-equivalent multiplication rate (e.g., 0.22 for 22%).
    pub rate: f64,
    /// Subtraction amount that, combined with the rate, yields the correct
    /// progressive tax.
    pub subtraction_amount: f64,
}

// ---------------------------------------------------------------------------
// CSV parsing
// ---------------------------------------------------------------------------

/// Parse a Tax Table CSV into a sorted vector of [`TaxTableRow`]s.
pub(crate) fn parse_tax_table(csv: &str) -> Vec<TaxTableRow> {
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

/// Parse a Tax Computation Worksheet CSV, returning only the brackets for the
/// given filing status.
pub(crate) fn parse_worksheet(csv: &str, status: FilingStatus) -> Vec<WorksheetBracket> {
    let key = filing_status_csv_key(status);
    csv.lines()
        .skip(1) // header
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 5 || cols[0] != key {
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

/// Map a [`FilingStatus`] to the corresponding key used in the CSV files.
fn filing_status_csv_key(status: FilingStatus) -> &'static str {
    match status {
        FilingStatus::Single => "single",
        FilingStatus::MarriedFilingJointly | FilingStatus::QualifyingSurvivingSpouse => {
            "married_filing_jointly"
        }
        FilingStatus::MarriedFilingSeparately => "married_filing_separately",
        FilingStatus::HeadOfHousehold => "head_of_household",
    }
}
