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

/// IRS filing status for Form 1040.
///
/// Filing status determines tax rates, standard deduction amounts, and eligibility
/// for certain credits and deductions.
///
/// See: <https://www.irs.gov/publications/p501#en_US_2024_publink1000220721>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilingStatus {
    /// Unmarried or legally separated/divorced on the last day of the tax year,
    /// and not qualifying for another filing status.
    Single,

    /// Married couples who agree to file a joint return, combining their income,
    /// deductions, and credits. Both spouses are jointly liable for the tax.
    MarriedFilingJointly,

    /// Married individuals who choose to file separate returns. May be beneficial
    /// when one spouse has significant medical expenses or miscellaneous deductions.
    MarriedFilingSeparately,

    /// Unmarried individuals who paid more than half the cost of keeping up a home
    /// for a qualifying person (such as a dependent child or parent).
    HeadOfHousehold,

    /// A surviving spouse whose spouse died during one of the two prior tax years
    /// and who has a dependent child. Allows use of joint return tax rates.
    QualifyingSurvivingSpouse,
}

impl fmt::Display for FilingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilingStatus::Single => write!(f, "Single"),
            FilingStatus::MarriedFilingJointly => write!(f, "Married Filing Jointly"),
            FilingStatus::MarriedFilingSeparately => write!(f, "Married Filing Separately"),
            FilingStatus::HeadOfHousehold => write!(f, "Head of Household"),
            FilingStatus::QualifyingSurvivingSpouse => {
                write!(f, "Qualifying Surviving Spouse")
            }
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
