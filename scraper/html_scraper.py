"""HTML scraping (BeautifulSoup) for the current-year IRS 1040 instructions."""

import os
import re

import requests
from bs4 import BeautifulSoup

from common import (
    FILING_STATUSES,
    HEADERS,
    HTML_URL,
    WORKSHEET_SECTION_LABELS,
    write_computation_worksheet_csv,
    write_tax_table_csv,
)


def detect_html_year(soup):
    """Detect which tax year the HTML instructions page covers.

    The page title is typically '1040 (2025) | Internal Revenue Service'.
    """
    title = soup.title.get_text() if soup.title else ""
    m = re.search(r"1040\s*\((\d{4})\)", title)
    return int(m.group(1)) if m else None


def parse_tax_table_html(soup):
    """Parse the Tax Table from the HTML page (income < $100,000)."""
    tax_table_heading = None
    for h2 in soup.find_all("h2"):
        if h2.get_text(strip=True) == "Tax Table":
            tax_table_heading = h2
            break
    if not tax_table_heading:
        raise ValueError("Could not find 'Tax Table' heading")

    big_table = None
    for table in tax_table_heading.find_all_next("table"):
        if len(table.find_all("tr")) > 100:
            big_table = table
            break
    if not big_table:
        raise ValueError("Could not find the large Tax Table")

    results = []
    for row in big_table.find_all("tr"):
        cells = [td.get_text(strip=True) for td in row.find_all(["th", "td"])]
        if len(cells) < 6:
            continue
        try:
            income_min = int(cells[0].replace(",", ""))
            income_max = int(cells[1].replace(",", ""))
        except (ValueError, IndexError):
            continue
        try:
            results.append(
                {
                    "income_min": income_min,
                    "income_max": income_max,
                    "single": int(cells[2].replace(",", "")),
                    "married_filing_jointly": int(cells[3].replace(",", "")),
                    "married_filing_separately": int(cells[4].replace(",", "")),
                    "head_of_household": int(cells[5].replace(",", "")),
                }
            )
        except ValueError:
            continue

    print(f"  Parsed {len(results)} tax table rows")
    return results


def parse_computation_worksheet_html(soup):
    """Parse the Tax Computation Worksheet from the HTML page (income >= $100,000)."""
    results = {}

    for section_label, filing_status in zip(WORKSHEET_SECTION_LABELS, FILING_STATUSES):
        elem = soup.find(
            string=lambda s, sl=section_label: s
            and sl in s
            and "filing status" in s.lower()
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

            rate_match = re.search(r"\((\d+\.\d+)\)", cells[2])
            if not rate_match:
                continue
            rate = float(rate_match.group(1))

            sub_match = re.search(r"\$\s*([\d,]+\.\d+)", cells[4])
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

    return results


def scrape_html(year, base_dir):
    """Scrape the current-year HTML instructions page.

    Returns True if the HTML page matched the requested year, False otherwise.
    """
    print(f"\nScraping {year} from HTML ({HTML_URL})")
    resp = requests.get(HTML_URL, headers=HEADERS, timeout=60)
    resp.raise_for_status()
    soup = BeautifulSoup(resp.text, "html.parser")

    html_year = detect_html_year(soup)
    if html_year != year:
        return False

    year_dir = os.path.join(base_dir, str(year))

    print("Parsing Tax Table...")
    tax_table = parse_tax_table_html(soup)
    write_tax_table_csv(tax_table, os.path.join(year_dir, "tax_table.csv"))

    print("Parsing Tax Computation Worksheet...")
    worksheet = parse_computation_worksheet_html(soup)
    write_computation_worksheet_csv(
        worksheet, os.path.join(year_dir, "tax_computation_worksheet.csv")
    )
    return True
