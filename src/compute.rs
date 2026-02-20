//! Core tax computation logic.

use crate::data;
use crate::types::{FilingStatus, TaxError, TaxYear};

/// Compute federal income tax for a given tax year, filing status, and taxable income.
///
/// # Arguments
///
/// * `year` — The tax year to use for bracket data.
/// * `status` — The taxpayer's filing status.
/// * `taxable_income` — Taxable income in whole dollars (typically Form 1040,
///   line 15). This is usually derived from adjusted gross income (AGI) or
///   modified adjusted gross income (MAGI) minus deductions.
///
/// # Returns
///
/// The computed federal income tax in whole dollars, rounded to the nearest
/// dollar consistent with IRS instructions.
///
/// # Method selection
///
/// - **Income < $100,000** — Uses the IRS Tax Table (a lookup table with
///   pre-computed values in $50 income increments). A binary search is used
///   to find the matching row.
///
/// - **Income >= $100,000** — Uses the Tax Computation Worksheet formula:
///   `tax = taxable_income × rate − subtraction_amount`.
///
/// # Errors
///
/// Returns [`TaxError::NegativeIncome`] if `taxable_income` is negative.
/// Returns [`TaxError::NoBracketFound`] if no matching bracket exists (should
/// not occur with valid embedded data).
///
/// # Examples
///
/// ```
/// use us_tax_brackets::{compute_tax, FilingStatus, TaxYear};
///
/// // Tax Table lookup (income under $100k)
/// let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 50_000).unwrap();
/// assert_eq!(tax, 5_920);
///
/// // Worksheet formula (income $100k+)
/// let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 150_000).unwrap();
/// assert_eq!(tax, 28_847);
/// ```
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
        TaxYear::Y2025 => (data::TAX_TABLE_CSV_2025, data::WORKSHEET_CSV_2025),
    };

    if taxable_income < 100_000 {
        compute_from_tax_table(table_csv, status, taxable_income)
    } else {
        compute_from_worksheet(worksheet_csv, status, taxable_income)
    }
}

/// Look up the tax in the IRS Tax Table (income < $100,000).
///
/// The table rows are sorted by `income_min` in $50 increments, so binary
/// search finds the matching row in O(log n).
fn compute_from_tax_table(
    csv: &str,
    status: FilingStatus,
    taxable_income: i64,
) -> Result<i64, TaxError> {
    let table = data::parse_tax_table(csv);

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
}

/// Compute tax using the Tax Computation Worksheet (income >= $100,000).
///
/// Iterates through the brackets for the given filing status and applies
/// `tax = income × rate − subtraction_amount` for the matching bracket.
fn compute_from_worksheet(
    csv: &str,
    status: FilingStatus,
    taxable_income: i64,
) -> Result<i64, TaxError> {
    let brackets = data::parse_worksheet(csv, status);

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

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Edge cases -----

    #[test]
    fn zero_income() {
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 0).unwrap();
        assert_eq!(tax, 0);
    }

    #[test]
    fn negative_income() {
        let result = compute_tax(TaxYear::Y2025, FilingStatus::Single, -1);
        assert_eq!(result, Err(TaxError::NegativeIncome));
    }

    // ----- Tax Table lookups (income < $100,000) -----

    #[test]
    fn low_income_single() {
        // $10 taxable income falls in the $5–$15 row -> $1 tax
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 10).unwrap();
        assert_eq!(tax, 1);
    }

    #[test]
    fn table_single_50k() {
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 50_000).unwrap();
        assert_eq!(tax, 5_920);
    }

    #[test]
    fn table_married_jointly_75k() {
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 75_000).unwrap();
        assert_eq!(tax, 8_526);
    }

    // ----- Tax Table / Worksheet boundary -----

    #[test]
    fn boundary_99999_uses_table() {
        // Last $50 bucket before $100k: row 99950–100000 -> single = 16909
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 99_999).unwrap();
        assert_eq!(tax, 16_909);
    }

    #[test]
    fn boundary_100k_uses_worksheet() {
        // "At least $100,000 but not over $103,350" -> 100000 × 0.22 − 5086 = 16914
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 100_000).unwrap();
        assert_eq!(tax, 16_914);
    }

    // ----- Worksheet computations (income >= $100,000) -----

    #[test]
    fn worksheet_single_150k() {
        // 150000 × 0.24 − 7153 = 28847
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 150_000).unwrap();
        assert_eq!(tax, 28_847);
    }

    #[test]
    fn worksheet_married_jointly_200k() {
        // 200000 × 0.22 − 10172 = 33828
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
        assert_eq!(tax, 33_828);
    }

    #[test]
    fn worksheet_head_of_household_300k() {
        // 300000 × 0.35 − 32191 = 72809
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::HeadOfHousehold, 300_000).unwrap();
        assert_eq!(tax, 72_809);
    }

    #[test]
    fn worksheet_single_1m() {
        // 1000000 × 0.37 − 42979.75 = 327020.25 -> 327020
        let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 1_000_000).unwrap();
        assert_eq!(tax, 327_020);
    }

    // ----- Cross-status comparison -----

    #[test]
    fn all_statuses_at_200k() {
        let single = compute_tax(TaxYear::Y2025, FilingStatus::Single, 200_000).unwrap();
        let mfj = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
        let mfs = compute_tax(
            TaxYear::Y2025,
            FilingStatus::MarriedFilingSeparately,
            200_000,
        )
        .unwrap();
        let hoh = compute_tax(TaxYear::Y2025, FilingStatus::HeadOfHousehold, 200_000).unwrap();

        // MFJ has the lowest tax at the same income level
        assert!(mfj < single);
        assert!(mfj < mfs);
        assert!(mfj < hoh);

        assert_eq!(single, 41_063); // 200000 × 0.32 − 22937
        assert_eq!(mfj, 33_828); //   200000 × 0.22 − 10172
        assert_eq!(mfs, 41_063); //   same brackets as single at this level
        assert_eq!(hoh, 39_324); //   200000 × 0.32 − 24676
    }
}
