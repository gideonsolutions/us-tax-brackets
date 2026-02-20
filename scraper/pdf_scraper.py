"""PDF scraping (pdfplumber) for prior-year IRS 1040 instructions."""

import os
import re
import tempfile

import pdfplumber
import requests

from common import (
    FILING_STATUSES,
    HEADERS,
    PDF_URL_TEMPLATE,
    write_computation_worksheet_csv,
    write_tax_table_csv,
)


def parse_int(s):
    """Parse an integer from a string like '1,234' or '1234'."""
    return int(s.replace(",", "").strip())


def parse_tax_table_pdf(pdf):
    """Parse the Tax Table from the PDF (income < $100,000)."""
    results = []
    in_tax_table = False

    for page in pdf.pages:
        text = page.extract_text() or ""

        if "Tax Table" in text and ("Your tax is" in text or "At But" in text):
            in_tax_table = True
        elif "Tax Computation Worksheet" in text:
            in_tax_table = False

        if not in_tax_table:
            continue

        for line in text.split("\n"):
            line = line.strip()
            if not line:
                continue

            parts = line.split()
            if len(parts) < 6:
                continue

            try:
                parse_int(parts[0])
            except ValueError:
                continue

            i = 0
            while i + 5 < len(parts):
                try:
                    income_min = parse_int(parts[i])
                    income_max = parse_int(parts[i + 1])
                    single = parse_int(parts[i + 2])
                    mfj = parse_int(parts[i + 3])
                    mfs = parse_int(parts[i + 4])
                    hoh = parse_int(parts[i + 5])

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


def parse_computation_worksheet_pdf(pdf):
    """Parse the Tax Computation Worksheet from the PDF (income >= $100,000)."""
    results = {}

    for page in pdf.pages:
        text = page.extract_text() or ""
        if "Tax Computation Worksheet" not in text:
            continue

        tables = page.extract_tables()
        worksheet_tables = [t for t in tables if len(t) == 6]

        if len(worksheet_tables) < 4:
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

                rate_cell = row[2] if len(row) > 2 else ""
                rate_match = re.search(r"\((\d+\.\d+)\)", rate_cell or "")
                if not rate_match:
                    continue
                rate = float(rate_match.group(1))

                sub_cell = row[4] if len(row) > 4 else ""
                sub_match = re.search(r"\$\s*([\d,]+\.\d+)", sub_cell or "")
                subtraction = (
                    float(sub_match.group(1).replace(",", "")) if sub_match else 0.0
                )

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

        break

    return results


def scrape_pdf(year, base_dir):
    """Scrape a prior-year PDF."""
    url = PDF_URL_TEMPLATE.format(year=year)
    print(f"\nScraping {year} from PDF ({url})")

    resp = requests.get(url, headers=HEADERS, timeout=120)
    resp.raise_for_status()
    tmp = tempfile.NamedTemporaryFile(suffix=".pdf", delete=False)
    tmp.write(resp.content)
    tmp.close()
    print(f"  Downloaded ({len(resp.content)} bytes)")

    try:
        with pdfplumber.open(tmp.name) as pdf:
            year_dir = os.path.join(base_dir, str(year))

            print("Parsing Tax Table...")
            tax_table = parse_tax_table_pdf(pdf)
            write_tax_table_csv(tax_table, os.path.join(year_dir, "tax_table.csv"))

            print("Parsing Tax Computation Worksheet...")
            worksheet = parse_computation_worksheet_pdf(pdf)
            write_computation_worksheet_csv(
                worksheet,
                os.path.join(year_dir, "tax_computation_worksheet.csv"),
            )
    finally:
        os.unlink(tmp.name)
