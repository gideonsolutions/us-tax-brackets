//! Public types: tax year, filing status, and error definitions.

use std::fmt;

/// A tax year supported by this crate.
///
/// Each variant corresponds to a set of IRS tax tables and computation
/// worksheet brackets embedded in the crate. New variants are added as
/// the IRS publishes updated instructions each year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxYear {
    /// Tax year 2025 (filed in 2026).
    Y2025,
}

impl fmt::Display for TaxYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaxYear::Y2025 => write!(f, "2025"),
        }
    }
}

/// Federal income tax filing status.
///
/// These correspond to the four filing status categories used by the IRS
/// on Form 1040. "Qualifying surviving spouse" uses the same brackets as
/// [`FilingStatus::MarriedFilingJointly`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilingStatus {
    /// Filing as an unmarried individual.
    Single,
    /// Filing jointly with a spouse, or as a qualifying surviving spouse.
    MarriedFilingJointly,
    /// Filing separately from a spouse.
    MarriedFilingSeparately,
    /// Filing as head of household (unmarried with qualifying dependents).
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

/// Errors that can occur during tax computation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaxError {
    /// The provided taxable income was negative.
    NegativeIncome,
    /// No matching tax bracket was found for the given income.
    ///
    /// This should not occur under normal usage and may indicate corrupted
    /// embedded data.
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
