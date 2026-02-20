//! Compute U.S. federal income tax from IRS tax tables and computation worksheets.
//!
//! This crate provides federal income tax computation based on official IRS data
//! scraped from the [Form 1040 instructions](https://www.irs.gov/instructions/i1040gi).
//!
//! # Overview
//!
//! The IRS provides two mechanisms for computing federal income tax:
//!
//! - **Tax Table** — For taxable incomes under $100,000. The IRS publishes a
//!   lookup table with pre-computed tax amounts in $50 income increments for each
//!   filing status.
//!
//! - **Tax Computation Worksheet** — For taxable incomes of $100,000 or more.
//!   Uses the formula: `tax = taxable_income × rate − subtraction_amount`, where
//!   the rate and subtraction amount depend on the income bracket and filing status.
//!
//! This crate embeds both datasets at compile time and exposes a single
//! [`compute_tax`] function that automatically selects the correct method.
//!
//! # Supported tax years
//!
//! | Year | Variant |
//! |------|---------|
//! | 2025 | [`TaxYear::Y2025`] |
//!
//! # Examples
//!
//! ```
//! use us_tax_brackets::{FilingStatus, TaxYear, compute_tax};
//!
//! // Single filer, $75,000 taxable income (uses Tax Table)
//! let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 75_000).unwrap();
//! assert_eq!(tax, 11_420);
//!
//! // Married filing jointly, $200,000 taxable income (uses Worksheet)
//! let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
//! assert_eq!(tax, 33_828);
//!
//! // Head of household, $300,000 taxable income
//! let tax = compute_tax(TaxYear::Y2025, FilingStatus::HeadOfHousehold, 300_000).unwrap();
//! assert_eq!(tax, 72_809);
//! ```
//!
//! # Data sources
//!
//! All tax data is scraped from the official IRS Form 1040 instructions using
//! the BeautifulSoup-based scraper included in the `scraper/` directory of the
//! repository. The CSV files are stored in `data/<year>/` and embedded into the
//! binary at compile time via [`include_str!`].

mod compute;
mod data;
mod types;

pub use compute::compute_tax;
pub use types::{FilingStatus, TaxError, TaxYear};
