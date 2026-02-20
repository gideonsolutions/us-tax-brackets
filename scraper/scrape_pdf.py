"""
Scrape IRS 1040 instructions PDF to extract:
1. Tax Table (income < $100,000)
2. Tax Computation Worksheet (income >= $100,000)

Outputs CSV files into data/<year>/ directory.

Usage:
    python scraper/scrape_pdf.py
"""

import csv
import os
import re
import tempfile

import pdfplumber
import requests

URLS = {
    2024: "https://www.irs.gov/pub/irs-prior/i1040gi--2024.pdf",
}

HEADERS = {"User-Agent": "Mozilla/5.0 (compatible; us-tax-brackets-scraper/1.0)"}

FILING_STATUSES = [
    "single",
    "married_filing_jointly",
    "married_filing_separately",
    "head_of_household",
]


def download_pdf(url):
    """Download PDF to a temporary file and return the path."""
    print(f"  Downloading {url}...")
    resp = requests.get(url, headers=HEADERS, timeout=120)
    resp.raise_for_status()
    tmp = tempfile.NamedTemporaryFile(suffix=".pdf", delete=False)
    tmp.write(resp.content)
    tmp.close()
    print(f"  Downloaded ({len(resp.content)} bytes)")
    return tmp.name


def parse_int(s):
    """Parse an integer from a string like '1,234' or '1234'."""
    return int(s.replace(",", "").strip())


def parse_tax_table_from_pdf(pdf):
    """Parse the Tax Table from the PDF.

    The PDF has a 3-column layout per page. pdfplumber extracts tables where
    the first data column contains newline-separated rows of
    "income_min income_max single_tax". The other columns contain the
    MFJ, MFS, and HOH values also newline-separated.

    We use extract_text() and parse the columnar text directly since the
    table extraction is unreliable for this layout.
    """
    results = []
    in_tax_table = False

    for page in pdf.pages:
        text = page.extract_text() or ""

        # Detect tax table pages
        if "Tax Table" in text and ("Your tax is" in text or "At But" in text):
            in_tax_table = True
        elif "Tax Computation Worksheet" in text:
            in_tax_table = False

        if not in_tax_table:
            continue

        # Extract lines and look for data rows: lines with numbers
        # Format: "income_min income_max single mfj mfs hoh"
        # or sometimes with additional columns from the 3-column layout
        for line in text.split("\n"):
            line = line.strip()
            if not line:
                continue

            # Try to match a tax table data line
            # These look like: "50,000 50,050 5,920 5,526 5,920 5,832"
            # or with multiple sets per line (3-column layout):
            # "50,000 50,050 5,920 5,526 5,920 5,832 53,000 53,050 ..."
            parts = line.split()
            if len(parts) < 6:
                continue

            # Check if first part looks like a number
            try:
                parse_int(parts[0])
            except ValueError:
                continue

            # Parse groups of 6 values from the line
            i = 0
            while i + 5 < len(parts):
                try:
                    income_min = parse_int(parts[i])
                    income_max = parse_int(parts[i + 1])
                    single = parse_int(parts[i + 2])
                    mfj = parse_int(parts[i + 3])
                    mfs = parse_int(parts[i + 4])
                    hoh = parse_int(parts[i + 5])

                    # Sanity check: income_max should be slightly > income_min.
                    # The early rows use increments of 5, 10, 25, or 50.
                    diff = income_max - income_min
                    if (
                        diff in (5, 10, 25, 50)
                        and income_max <= 100000
                        and income_min >= 0
                    ):
                        results.append(
                            {
                                "income_min": income_min,
                                "income_max": income_max,
                                "single": single,
                                "married_filing_jointly": mfj,
                                "married_filing_separately": mfs,
                                "head_of_household": hoh,
                            }
                        )
                        i += 6
                    else:
                        i += 1
                except (ValueError, IndexError):
                    i += 1

    # Sort and deduplicate
    results.sort(key=lambda r: r["income_min"])
    seen = set()
    deduped = []
    for r in results:
        key = (r["income_min"], r["income_max"])
        if key not in seen:
            seen.add(key)
            deduped.append(r)

    print(f"  Parsed {len(deduped)} tax table rows")
    return deduped


def parse_computation_worksheet_from_pdf(pdf):
    """Parse the Tax Computation Worksheet from the PDF.

    The worksheet tables appear on a single page with 4 tables
    (Sections A-D) for each filing status.
    """
    results = {}

    for page in pdf.pages:
        text = page.extract_text() or ""
        if "Tax Computation Worksheet" not in text:
            continue

        tables = page.extract_tables()
        # Filter to worksheet tables (6 rows: 1 header + 5 brackets)
        worksheet_tables = [t for t in tables if len(t) == 6]

        if len(worksheet_tables) < 4:
            # Sometimes the last bracket row has an extra row
            worksheet_tables = [t for t in tables if 5 <= len(t) <= 7]

        if len(worksheet_tables) < 4:
            continue

        for filing_status, table in zip(FILING_STATUSES, worksheet_tables[:4]):
            brackets = []
            for row in table:
                if not row or not row[0]:
                    continue
                range_text = row[0]
                if "100,000" not in range_text and "Over" not in range_text:
                    continue

                # Parse rate: "× 22% (0.22)" -> 0.22
                rate_cell = row[2] if len(row) > 2 else ""
                rate_match = re.search(r"\((\d+\.\d+)\)", rate_cell or "")
                if not rate_match:
                    continue
                rate = float(rate_match.group(1))

                # Parse subtraction amount: "$ 5,086.00" -> 5086.00
                sub_cell = row[4] if len(row) > 4 else ""
                sub_match = re.search(r"\$\s*([\d,]+\.\d+)", sub_cell or "")
                subtraction = (
                    float(sub_match.group(1).replace(",", "")) if sub_match else 0.0
                )

                # Parse income range
                income_min = None
                income_max = None
                min_match = re.search(r"\$\s*([\d,]+)", range_text)
                if min_match:
                    income_min = int(min_match.group(1).replace(",", ""))
                max_match = re.search(r"not over \$\s*([\d,]+)", range_text)
                if max_match:
                    income_max = int(max_match.group(1).replace(",", ""))
                if "Over" in range_text and income_max is None:
                    over_match = re.search(r"Over \$\s*([\d,]+)", range_text)
                    if over_match:
                        income_min = int(over_match.group(1).replace(",", ""))

                brackets.append(
                    {
                        "income_min": income_min,
                        "income_max": income_max,
                        "rate": rate,
                        "subtraction_amount": subtraction,
                    }
                )

            results[filing_status] = brackets
            print(f"  {filing_status}: {len(brackets)} brackets")

        break  # Found the worksheet page

    return results


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


def main():
    base_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "data")

    for year, url in URLS.items():
        print(f"\nScraping {year} from PDF")
        pdf_path = download_pdf(url)

        try:
            with pdfplumber.open(pdf_path) as pdf:
                year_dir = os.path.join(base_dir, str(year))

                print("Parsing Tax Table...")
                tax_table = parse_tax_table_from_pdf(pdf)
                write_tax_table_csv(tax_table, os.path.join(year_dir, "tax_table.csv"))

                print("Parsing Tax Computation Worksheet...")
                worksheet = parse_computation_worksheet_from_pdf(pdf)
                write_computation_worksheet_csv(
                    worksheet,
                    os.path.join(year_dir, "tax_computation_worksheet.csv"),
                )
        finally:
            os.unlink(pdf_path)

    print("\nDone!")


if __name__ == "__main__":
    main()
