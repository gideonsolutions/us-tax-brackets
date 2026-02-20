"""Scrape IRS 1040 instructions to extract tax data.

Automatically uses HTML (BeautifulSoup) if the IRS instructions page matches the
requested tax year, otherwise falls back to the prior-year PDF (pdfplumber).

Outputs CSV files into data/<year>/ directory.

Usage:
    pip install beautifulsoup4 pdfplumber requests
    python scraper/scrape.py            # scrape all years
    python scraper/scrape.py 2023       # scrape a single year
"""

import os
import sys

from common import TAX_YEARS
from html_scraper import scrape_html
from pdf_scraper import scrape_pdf


def main():
    base_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "data")
    years = TAX_YEARS if len(sys.argv) < 2 else [int(y) for y in sys.argv[1:]]

    for year in years:
        # Try HTML first — if the IRS page matches this year, use it
        if not scrape_html(year, base_dir):
            # HTML page is for a different year, fall back to PDF
            scrape_pdf(year, base_dir)

    print("\nDone!")


if __name__ == "__main__":
    main()
