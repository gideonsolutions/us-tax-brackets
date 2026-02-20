# us-tax-brackets

[![Crates.io](https://img.shields.io/crates/v/us-tax-brackets)](https://crates.io/crates/us-tax-brackets)
[![Rust](https://github.com/gideonsolutions/us-tax-brackets/actions/workflows/rust.yml/badge.svg)](https://github.com/gideonsolutions/us-tax-brackets/actions/workflows/rust.yml)
[![Python](https://github.com/gideonsolutions/us-tax-brackets/actions/workflows/python.yml/badge.svg)](https://github.com/gideonsolutions/us-tax-brackets/actions/workflows/python.yml)
[![License](https://img.shields.io/crates/l/us-tax-brackets)](LICENSE)

Compute U.S. federal income tax from official IRS tax tables and computation worksheets.

## Overview

The IRS provides two mechanisms for computing federal income tax on Form 1040:

- **Tax Table** — For taxable incomes under $100,000. A lookup table with pre-computed tax amounts in $50 income increments for each filing status.
- **Tax Computation Worksheet** — For taxable incomes of $100,000 or more. Uses the formula: `tax = taxable_income * rate - subtraction_amount`.

This crate embeds both datasets at compile time and exposes a single `compute_tax` function that automatically selects the correct method.

## Installation

```sh
cargo add us-tax-brackets
```

## Usage

```rust
use us_tax_brackets::{compute_tax, FilingStatus, TaxYear};

// Single filer, $75,000 taxable income (uses Tax Table)
let tax = compute_tax(TaxYear::Y2025, FilingStatus::Single, 75_000).unwrap();
assert_eq!(tax, 11_420);

// Married filing jointly, $200,000 taxable income (uses Worksheet)
let tax = compute_tax(TaxYear::Y2025, FilingStatus::MarriedFilingJointly, 200_000).unwrap();
assert_eq!(tax, 33_828);

// Head of household, $300,000 taxable income
let tax = compute_tax(TaxYear::Y2025, FilingStatus::HeadOfHousehold, 300_000).unwrap();
assert_eq!(tax, 72_809);
```

## Filing statuses

| Variant | Description |
|---------|-------------|
| `Single` | Unmarried or legally separated/divorced |
| `MarriedFilingJointly` | Married couples filing a joint return |
| `MarriedFilingSeparately` | Married individuals filing separate returns |
| `HeadOfHousehold` | Unmarried with qualifying dependents |
| `QualifyingSurvivingSpouse` | Surviving spouse with dependent child (uses joint return rates) |

## Supported tax years

| Year | Variant | Source |
|------|---------|--------|
| 2023 | `TaxYear::Y2023` | PDF (prior year) |
| 2024 | `TaxYear::Y2024` | PDF (prior year) |
| 2025 | `TaxYear::Y2025` | HTML (current year) |

## Data sources

All tax data is scraped from the official IRS Form 1040 instructions. The current year (2025) is scraped from the [HTML instructions page](https://www.irs.gov/instructions/i1040gi) using BeautifulSoup. Prior years are scraped from the [PDF instructions](https://www.irs.gov/pub/irs-prior/) using pdfplumber. The CSV files are stored in `data/<year>/` and embedded into the binary at compile time via `include_str!`.

### Updating data

The unified scraper automatically uses HTML for the current year and falls back to PDF for prior years:

```sh
pip install beautifulsoup4 pdfplumber requests
python scraper/scrape.py            # scrape all years
python scraper/scrape.py 2023       # scrape a single year
```

## License

Apache-2.0
