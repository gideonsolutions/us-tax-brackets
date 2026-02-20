"""
Scrape IRS 1040 instructions to extract:
1. Tax Table (income < $100,000)
2. Tax Computation Worksheet (income >= $100,000)

Outputs CSV files into data/<year>/ directory.
"""
import csv
import os
import re
import sys

import requests
from bs4 import BeautifulSoup

# Map of tax year -> IRS instructions URL
# The "i1040gi" page is for the current year; prior years use "i1040gi/ar01.html" or similar.
# We can also parametrize: https://www.irs.gov/instructions/i1040gi
URLS = {
    2025: "https://www.irs.gov/instructions/i1040gi",
}

HEADERS = {"User-Agent": "Mozilla/5.0 (compatible; irs-tax-brackets-scraper/1.0)"}

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


def fetch_soup(url: str) -> BeautifulSoup:
    resp = requests.get(url, headers=HEADERS, timeout=60)
    resp.raise_for_status()
    return BeautifulSoup(resp.text, "html.parser")


def parse_tax_table(soup: BeautifulSoup) -> list[dict]:
    """Parse the Tax Table (income under $100,000).

    Returns list of dicts with keys:
        income_min, income_max, single, married_filing_jointly,
        married_filing_separately, head_of_household
    """
    # Find the big table (2000+ rows)
    tax_table_heading = None
    for h2 in soup.find_all("h2"):
        if h2.get_text(strip=True) == "Tax Table":
            tax_table_heading = h2
            break
    if not tax_table_heading:
        raise ValueError("Could not find 'Tax Table' heading")

    # The large table is the one with > 100 rows after the heading
    big_table = None
    for table in tax_table_heading.find_all_next("table"):
        rows = table.find_all("tr")
        if len(rows) > 100:
            big_table = table
            break
    if not big_table:
        raise ValueError("Could not find the large Tax Table")

    results = []
    for row in big_table.find_all("tr"):
        cells = [td.get_text(strip=True) for td in row.find_all(["th", "td"])]
        if len(cells) < 6:
            continue
        # Skip header rows
        try:
            income_min = int(cells[0].replace(",", ""))
            income_max = int(cells[1].replace(",", ""))
        except (ValueError, IndexError):
            continue
        try:
            results.append({
                "income_min": income_min,
                "income_max": income_max,
                "single": int(cells[2].replace(",", "")),
                "married_filing_jointly": int(cells[3].replace(",", "")),
                "married_filing_separately": int(cells[4].replace(",", "")),
                "head_of_household": int(cells[5].replace(",", "")),
            })
        except ValueError:
            continue

    print(f"  Parsed {len(results)} tax table rows")
    return results


def parse_computation_worksheet(soup: BeautifulSoup) -> dict[str, list[dict]]:
    """Parse the Tax Computation Worksheet (income >= $100,000).

    Returns dict mapping filing_status -> list of bracket dicts with keys:
        income_min, income_max (or None for unbounded), rate, subtraction_amount
    """
    results = {}

    for section_label, filing_status in zip(WORKSHEET_SECTION_LABELS, FILING_STATUSES):
        # Find the text containing the section label
        elem = soup.find(
            string=lambda s, sl=section_label: s and sl in s and "filing status" in s.lower()
        )
        if not elem:
            raise ValueError(f"Could not find {section_label}")

        parent = elem.find_parent()
        table = parent.find_next("table") if parent else None
        if not table:
            raise ValueError(f"Could not find table for {section_label}")

        brackets = []
        for row in table.find_all("tr"):
            cells = [td.get_text(strip=True) for td in row.find_all(["th", "td"])]
            if len(cells) < 5:
                continue
            range_text = cells[0]
            if "100,000" not in range_text and "Over" not in range_text:
                continue

            # Parse rate: "× 22% (0.22)" -> 0.22
            rate_match = re.search(r"\((\d+\.\d+)\)", cells[2])
            if not rate_match:
                continue
            rate = float(rate_match.group(1))

            # Parse subtraction amount: "$ 5,086.00" -> 5086.00
            sub_match = re.search(r"\$\s*([\d,]+\.\d+)", cells[4])
            subtraction = float(sub_match.group(1).replace(",", "")) if sub_match else 0.0

            # Parse income range
            income_min = None
            income_max = None
            # "At least $100,000 but not over $103,350"
            min_match = re.search(r"\$\s*([\d,]+)", range_text)
            if min_match:
                income_min = int(min_match.group(1).replace(",", ""))
            max_match = re.search(r"not over \$\s*([\d,]+)", range_text)
            if max_match:
                income_max = int(max_match.group(1).replace(",", ""))
            # "Over $626,350" (no upper bound)
            if "Over" in range_text and income_max is None:
                over_match = re.search(r"Over \$\s*([\d,]+)", range_text)
                if over_match:
                    income_min = int(over_match.group(1).replace(",", ""))

            brackets.append({
                "income_min": income_min,
                "income_max": income_max,
                "rate": rate,
                "subtraction_amount": subtraction,
            })

        results[filing_status] = brackets
        print(f"  {filing_status}: {len(brackets)} brackets")

    return results


def write_tax_table_csv(data: list[dict], path: str):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=[
            "income_min", "income_max",
            "single", "married_filing_jointly",
            "married_filing_separately", "head_of_household",
        ])
        writer.writeheader()
        writer.writerows(data)
    print(f"  Wrote {path}")


def write_computation_worksheet_csv(data: dict[str, list[dict]], path: str):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=[
            "filing_status", "income_min", "income_max",
            "rate", "subtraction_amount",
        ])
        writer.writeheader()
        for filing_status, brackets in data.items():
            for bracket in brackets:
                row = {"filing_status": filing_status, **bracket}
                # Use empty string for unbounded max
                if row["income_max"] is None:
                    row["income_max"] = ""
                writer.writerow(row)
    print(f"  Wrote {path}")


def main():
    base_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "data")

    for year, url in URLS.items():
        print(f"\nScraping {year} from {url}")
        soup = fetch_soup(url)

        year_dir = os.path.join(base_dir, str(year))

        print("Parsing Tax Table...")
        tax_table = parse_tax_table(soup)
        write_tax_table_csv(tax_table, os.path.join(year_dir, "tax_table.csv"))

        print("Parsing Tax Computation Worksheet...")
        worksheet = parse_computation_worksheet(soup)
        write_computation_worksheet_csv(
            worksheet, os.path.join(year_dir, "tax_computation_worksheet.csv")
        )

    print("\nDone!")


if __name__ == "__main__":
    main()
