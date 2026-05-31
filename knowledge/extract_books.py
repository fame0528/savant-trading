import fitz
import os
import json
import sys

books_dir = r"C:\Users\spenc\dev\savant-trading\knowledge\books"
output_dir = r"C:\Users\spenc\dev\savant-trading\knowledge\extracted"

os.makedirs(output_dir, exist_ok=True)

books = [
    "Day Trading and Swing Trading the Currency Market - Technical and Fundamental Strategies to Profit from Market Moves 2nd edition 2008.pdf",
    "Currency Trading and Intermarket Analysis - How to Profit from the Shifting Currents in Global Markets 2008.pdf",
    "Inside the Currency Market - Mechanics, Valuation and Strategies 2011.pdf",
    "Sentiment in the Forex Market - Indicators and Strategies To Profit from Crowd Behavior and Market Extremes 2008.pdf",
    "Forex Patterns and Probabilities - Trading Strategies for Trending and Range-Bound Markets 2007.pdf",
    "Attacking Currency Trends - How to Anticipate and Trade Big Moves in the Forex Market 2011.pdf",
    "Naked Forex - High-Probability Techniques for Trading Without Indicators 2012.pdf",
    "The Little Book of Currency Trading - How to Make Big Profits in the World of Forex 2010.pdf",
    "The Sensible Guide to Forex - Safer, Smarter Ways to Survive and Prosper from the Start 2012.pdf",
    "Essentials of Foreign Exchange Trading 2009.pdf",
    "How to Make a Living Trading Foreign Exchange - A Guaranteed Income for Life 2010.pdf",
    "Currency Strategy - The Practitioners Guide to Currency Investing, Hedging and Forecasting 2002.pdf",
    "17 Proven Currency Trading Strategies - How to Profit in the Forex Market 2013.pdf",
    "Forex Trading Basics & Secrets Volume 3.0.pdf",
    "Forex for Beginners - A Comprehensive Guide to Profiting from the Global Currency Markets 2012.pdf",
    "Currency Trading For Dummies 2nd edition 2011.pdf",
    "The 10 Essentials of Forex Trading - The Rules for Turning Trading Patterns Into Profit 2007.pdf",
    "Forex Trading Secrets - Trading Strategies for the Forex Market 2010.pdf",
]

for i, book in enumerate(books):
    path = os.path.join(books_dir, book)
    if not os.path.exists(path):
        print(f"[{i+1}] MISSING: {book}")
        continue
    
    try:
        doc = fitz.open(path)
        text = ""
        for page_num in range(len(doc)):
            page = doc[page_num]
            text += page.get_text()
        num_pages = len(doc)
        doc.close()
        
        out_file = os.path.join(output_dir, f"book_{i+1:02d}.txt")
        with open(out_file, "w", encoding="utf-8") as f:
            f.write(text)
        
        print(f"[{i+1}] OK ({len(text)} chars, {num_pages} pages): {book[:80]}")
    except Exception as e:
        print(f"[{i+1}] ERROR: {book[:60]} - {e}")

print("\nDone extracting all books.")
