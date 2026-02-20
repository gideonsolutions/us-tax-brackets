"""Shared constants and CSV writers for the IRS scraper."""

import csv
import os

# The IRS HTML instructions page always shows the current year.
HTML_URL = "https://www.irs.gov/instructions/i1040gi"

# Prior-year PDFs follow this pattern.
PDF_URL_TEMPLATE = "https://www.irs.gov/pub/irs-prior/i1040gi--{year}.pdf"

# Tax years to scrape. Add new years here.
TAX_YEARS = [2023, 2024, 2025]

HEADERS = {"User-Agent": "Mozilla/5.0 (compatible; us-tax-brackets-scraper/1.0)"}

FILING_STATUSES = [
    "single",
    "married_filing_jointly",
    "married_filing_separately",
    "head_of_household",
]

WORKSHEET_SECTION_LABELS = [
    "Section A",  # Single
    "Section B",  # Married filing jointly / Qualifying surviving spouse
    "Section C",  # Married filing separately
    "Section D",  # Head of household
]


def write_tax_table_csv(data, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "income_min",
                "income_max",
                "single",
                "married_filing_jointly",
                "married_filing_separately",
                "head_of_household",
            ],
        )
        writer.writeheader()
        writer.writerows(data)
    print(f"  Wrote {path}")


def write_computation_worksheet_csv(data, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "filing_status",
                "income_min",
                "income_max",
                "rate",
                "subtraction_amount",
            ],
        )
        writer.writeheader()
        for filing_status, brackets in data.items():
            for bracket in brackets:
                row = {"filing_status": filing_status, **bracket}
                if row["income_max"] is None:
                    row["income_max"] = ""
                writer.writerow(row)
    print(f"  Wrote {path}")
