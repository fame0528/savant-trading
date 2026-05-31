import json

knowledge_units = []
uid = 0

def add(source, topic, conditions, content, priority=3):
    global uid
    uid += 1
    knowledge_units.append({
        "id": f"book-fx-{uid:03d}",
        "source": source,
        "topic": topic,
        "conditions": conditions,
        "content": content,
        "priority": priority
    })

# =========================================================================
# BOOK 1: Kathy Lien - Day Trading and Swing Trading the Currency Market
# =========================================================================

add("kathy-lien-currency-market", "Execution", ["SessionOpen", "SessionClose"],
    "Asian session (7PM-4AM EST): USD/JPY avg range 78 pips, GBP/JPY 112 pips. Best for JPY pairs. Thin liquidity allows large players to run stops. Risk-averse traders should trade AUD/USD, NZD/USD (38-42 pip range).", 5)

add("kathy-lien-currency-market", "Execution", ["SessionOpen"],
    "London session (2AM-12PM EST): Largest FX center (>30% volume). GBP/JPY and GBP/CHF reach 140-146 pip ranges. Half of 12 major pairs exceed 80 pips. Best for breakout and trend-following strategies.", 5)

add("kathy-lien-currency-market", "Execution", ["SessionOpen"],
    "US-European overlap (8AM-12PM EST) accounts for 70% of European session range and 80% of US session range. If you can only trade 4 hours/day, trade this window. Maximum liquidity and volatility convergence.", 5)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Multiple time frame analysis: Use daily charts to identify trend direction, hourly charts for entry levels. 70% of a market's move occurs 20% of the time. Trade in direction of higher TF trend for highest probability.", 5)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Ranging"],
    "Fading double zeros: Enter contra-trend at round numbers (e.g. 1.2800, 105.00). Place entry 5-10 pips beyond figure, stop 20 pips away. Take half profit at 2x risk, trail rest. Works best when round number aligns with Fibonacci or MA confluence.", 5)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["SessionOpen"],
    "Waiting for the Real Deal (GBP/USD London open): Watch for fake breakout in first 1-2 hours of London. Wait for reversal through Frankfurt-London range. Enter 10 pips beyond range, stop 20 pips. UK dealers are notorious stop hunters at open.", 5)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["HighVolatility", "LowVolatility"],
    "Inside day breakout: Need 2+ inside days on daily chart. Buy 10 pips above or sell 10 pips below prior inside day range. Stop-and-reverse at opposite side. Higher success in tight range pairs: EUR/GBP, USD/CAD, EUR/CHF.", 5)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Ranging"],
    "The Fader strategy: ADX must be <35 (range-bound). Wait for 15-pip break below prior day low, then enter long 15 pips above prior day high. Stop 30 pips, target 60 pips (2:1 R:R). Avoid on major news days.", 4)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending"],
    "Perfect Order: Sequential MA stack (10>20>50>100>200 SMA for uptrend). Enter 5 candles after formation. ADX should be >20 and rising. Exit when perfect order breaks (10 SMA crosses below 20). Captured 550 pips on EUR/USD example.", 4)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Channel strategy: Draw parallel trendlines containing price. Enter on breakout above upper channel line. Stop just below channel. Works especially well before London/NY opens and ahead of major economic releases.", 4)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending"],
    "Filtering false breakouts: Look for 20-day high, then 2-day low within 3 days, then new 20-day high within 3 more days. Enter on the re-break. This flushes weak hands before real trend continuation. 722-pip example on GBP/USD.", 4)

add("kathy-lien-currency-market", "RiskManagement", ["Trending", "Ranging"],
    "Two-day low stop method: Place stop 10 pips below the two-day low for longs. When profit reaches 2x risk, close half and move stop to breakeven. Trail remaining with parabolic SAR or 2-bar low method.", 5)

add("kathy-lien-currency-market", "RiskManagement", ["Trending", "Ranging"],
    "Never risk more than 2% of equity per trade. Maintain 2:1 reward-to-risk minimum. Never add to losing positions. Know your max acceptable drawdown before starting. Keep position sizes within reason.", 5)

add("kathy-lien-currency-market", "Sentiment", ["BreakingNews"],
    "News trading proactive method: Enter 15-20 min before release when spreads are still tight. Stop 10 pips below range low or 30 pips from entry. If wrong, stopped out immediately. If right, take half profit at 2x risk.", 5)

add("kathy-lien-currency-market", "MacroAnalysis", ["FomcDate", "BreakingNews"],
    "Top market-moving US data (20-min reaction): 1) Nonfarm Payrolls 2) FOMC Rate Decision 3) CPI 4) Retail Sales 5) PPI. Daily reaction differs: ISM Non-Manufacturing rises to #2. GDP is no longer a big deal (components known in advance).", 5)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending", "Ranging"],
    "Chart economic surprises: Stack positive vs negative data surprises against price. When price diverges from fundamentals for 1+ month, expect violent correction. Michael Steinhardt's variant perception method generated 24% avg returns for 30 years.", 4)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending"],
    "Interest rate differentials drive long-term currency trends. Monitor 10-year government bond yield spreads between countries. Capital flows to highest-yielding assets. Fed funds futures and Euribor futures price in future rate expectations.", 4)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Currency correlations change over time. EUR/USD and USD/CHF have -0.94 monthly correlation. USD/JPY and USD/CHF have +0.83. Taking opposite positions on correlated pairs doubles exposure without realizing it.", 4)

add("kathy-lien-currency-market", "RegimeDetection", ["Trending", "Ranging"],
    "Trending checklist: ADX>25, price above 50/100/200 SMA, Bollinger bands expanding, RSI between 40-80. Range-bound: ADX<25 trending down, oscillating RSI, contracting Bollingers. Never pick tops in trends or buy breakouts in ranges.", 5)

add("kathy-lien-currency-market", "RegimeDetection", ["Trending", "Ranging"],
    "Seasonality patterns: USD/JPY tends to rise in January and July, fall in August. USD/CHF and GBP/USD show dollar weakness in September. NZD/USD tends to appreciate in November. AUD/USD and NZD/USD weaken in May.", 3)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending"],
    "Trade flows: Net exporters have currency appreciation tendency (trade surplus = more demand for currency). Net importers see depreciation. Japan's trade surplus gives JPY natural bid despite zero interest rates.", 4)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending"],
    "Capital flows: Equity market rallies attract foreign capital, strengthening currency. Fixed income flows chase highest yields. TIC data measures foreign purchases of US assets - became key USD indicator after 2002.", 3)

add("kathy-lien-currency-market", "Psychology", ["Trending", "Ranging"],
    "10 trading rules: 1) Limit losses 2) Let profits run 3) Keep position sizes reasonable 4) Know R:R ratio 5) Be adequately capitalized 6) Don't fight trend 7) Never add to losers 8) Know market expectations 9) Keep journal 10) Have max loss threshold.", 5)

add("kathy-lien-currency-market", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Emotional detachment: Good traders are dedicated but not emotionally married to trades. They accept losses and make decisions intellectually. During losing streaks, take a break - continuing to trade breeds greater losses.", 4)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending"],
    "PPP (Purchasing Power Parity): Currencies revert to PPP over long term. If a Big Mac costs more in one country, its currency is overvalued. Useful for identifying long-term misalignment, not short-term trades.", 3)

add("kathy-lien-currency-market", "Execution", ["FomcDate", "BreakingNews"],
    "FOMC announcements occur at 2:15 PM NY time, past London close. London traders often position ahead, creating moves in European session. The day after FOMC often sees continuation or reversal - wait for direction to clarify.", 4)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending"],
    "Bollinger Band squeeze: When bands contract sharply, a breakout is imminent. Trade in direction of the breakout. Combine with ADX confirmation - if ADX rises above 25 during breakout, trend is genuine.", 4)

# =========================================================================
# BOOK 2: Lien - Currency Trading and Intermarket Analysis
# =========================================================================

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Dollar Smile Theory: USD strengthens in two scenarios - 1) US economy outperforming (risk-on, USD as growth currency) and 2) Global risk aversion flight-to-safety. USD weakest in middle scenario - US economy weak but no global crisis.", 5)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending", "HighVolatility"],
    "Intermarket correlations: DXY negatively correlated with gold (-0.80 to -0.90 historically). Crude oil and CAD positive correlation. When correlations break down, regime change is occurring - reduce position sizes.", 5)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Carry trade: Borrow low-yield currencies (JPY, CHF) to buy high-yielders (AUD, NZD). Works in stable/trending markets. Unwinds violently during risk aversion. AUD/JPY is the classic carry pair - tracks risk sentiment.", 5)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "US Treasury yields and USD/JPY have strong positive correlation. When 10-year yields rise, USD/JPY tends to rise. Monitor yield spreads: US-Japan 10yr spread is primary driver of USD/JPY direction.", 4)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Gold and AUD/USD positive correlation (both anti-USD, Australia is major gold exporter). When gold rallies, AUD tends to follow. Useful as confirmation signal - divergence warns of potential reversal.", 4)

add("kathy-lien-intermarket", "MacroAnalysis", ["ExtremeFear", "HighVolatility"],
    "VIX and forex: Rising VIX correlates with JPY and CHF strength (safe havens). Falling VIX favors commodity currencies (AUD, NZD, CAD). VIX above 30 signals extreme fear - expect carry trade unwinds.", 5)

add("kathy-lien-intermarket", "Sentiment", ["Trending"],
    "COT report analysis: When speculators reach extreme net long/short positions (top/bottom 5% of 3-year range), reversal is likely. Commercials are wrong at extremes but right at turning points - follow their positioning.", 5)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Commodity currencies: AUD tracks gold and base metals. CAD tracks crude oil. NZD tracks dairy prices. Monitor commodity indices as leading indicators for these currencies.", 4)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Equity-FX link: Rising US equities + rising USD = risk-on, US outperformance. Rising equities + falling USD = global risk-on, USD funding currency. Falling equities + rising USD = risk-off flight to safety.", 4)

add("kathy-lien-intermarket", "MacroAnalysis", ["FomcDate"],
    "Central bank divergence is the strongest FX trend driver. When Fed is tightening while ECB is easing, EUR/USD trends lower aggressively. Monitor rate expectations via OIS and futures curves.", 5)

add("kathy-lien-intermarket", "MacroAnalysis", ["BreakingNews"],
    "Economic surprise indices: When a country's data consistently beats expectations, its currency strengthens. Bloomberg Economic Surprise Index is key tool. Reversion to mean after extreme readings.", 4)

add("kathy-lien-intermarket", "RegimeDetection", ["Trending", "Ranging"],
    "Three market regimes: 1) Trending with momentum - follow trend 2) Range-bound mean reversion - fade extremes 3) Transitioning - reduce size. Identify regime by ADX, Bollinger bandwidth, and correlation stability.", 4)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "DXY (Dollar Index) is weighted: EUR 57.6%, JPY 13.6%, GBP 11.9%, CAD 9.1%, SEK 4.2%, CHF 3.6%. EUR/USD moves dominate DXY. DXY breakout/breakdown confirms or denies EUR/USD signals.", 4)

# =========================================================================
# BOOK 3: Brian Twomey - Inside the Currency Market
# =========================================================================

add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Interest rate parity: Forward rate = Spot rate x (1+domestic rate)/(1+foreign rate). If parity violated, arbitrage exists. Forward points reflect interest rate differentials. Widening spreads indicate trend acceleration.", 4)

add("twomey-inside-currency", "Execution", ["SessionOpen", "SessionClose"],
    "London fixing (4PM GMT) is the most important daily event for institutional FX. Large orders cluster around fix. Price often reverses after fix. Day traders should be flat or have stops below fix levels.", 4)

add("twomey-inside-currency", "OrderFlow", ["SessionOpen", "SessionClose"],
    "Bid-ask spread analysis: Narrowing spreads indicate increasing liquidity and trending conditions. Widening spreads signal uncertainty or low liquidity. Trade breakouts when spreads narrow, fade moves when spreads widen.", 4)

add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Balance of payments framework: Current account deficits must be financed by capital account surpluses. If capital inflows insufficient, currency depreciates. US twin deficits (trade + budget) are structural USD headwind.", 4)

add("twomey-inside-currency", "MacroAnalysis", ["FomcDate"],
    "Taylor Rule estimation: Optimal rate = neutral rate + 0.5*(inflation-target) + 0.5*(output gap). When market rates deviate significantly from Taylor Rule implied rate, expect central bank policy shift.", 4)

add("twomey-inside-currency", "TechnicalAnalysis", ["Trending"],
    "Fibonacci clusters: When multiple Fibonacci retracement levels from different waves converge at same price, that zone is extremely significant support/resistance. Higher cluster density = higher probability reversal zone.", 4)

add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Money supply growth differentials: M2 growth exceeding GDP growth = inflationary, currency bearish. Monitor M2 velocity changes. Quantitative easing expands money supply, diluting currency value.", 3)

add("twomey-inside-currency", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Pivot point trading: Calculate daily pivots (H+L+C)/3. Buy at S1 support with stop at S2, target R1. In strong trends, price often reaches R2 or S2. Pivot confluence with round numbers increases significance.", 3)

add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Carry trade mechanics: Total return = spot return + interest rate differential + roll yield. Negative carry erodes profits in sideways markets. Carry is positive convexity - small gains most days, occasional large losses.", 5)

add("twomey-inside-currency", "Execution", ["SessionOpen"],
    "Tokyo fix (9:55 AM JST) matters for JPY pairs. Japanese exporters and institutional orders cluster at fix. Option barriers near round numbers (100, 105, 110) create magnetic price action ahead of NY cut.", 3)

add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Real interest rates (nominal rate minus inflation) are the true driver of capital flows. Country with highest real rates attracts capital, strengthening currency. Monitor breakeven inflation rates from TIPS markets.", 4)

# =========================================================================
# BOOK 4: Jamie Saettele - Sentiment in the Forex Market
# =========================================================================

add("saettele-sentiment-forex", "Sentiment", ["ExtremeGreed", "ExtremeFear"],
    "Magazine cover indicator: When mainstream media features a currency trend on the cover (The Economist, Time), the trend is near exhaustion. 'Euroshambles' cover (Sept 2000) marked EUR/USD all-time low. Contrarian signal at extremes.", 5)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeGreed", "ExtremeFear"],
    "News headline contrarian signals: Extreme language (surge, plummet, plunge) in WSJ/Bloomberg headlines signals short-term reversal. Search 'dollar surge' - when articles spike, dollar is near short-term top.", 5)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeFear", "ExtremeGreed"],
    "COT report: Speculators net positioning at 3-year extremes (>90th or <10th percentile) signals reversal. When commercials and speculators both extreme in same direction, signal is strongest. Use as filter, not timing tool.", 5)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeGreed", "ExtremeFear"],
    "RSI as sentiment indicator: Don't use traditional overbought/oversold. Instead, use RSI divergences at extremes. RSI making lower highs while price makes higher highs = bearish divergence at sentiment extreme.", 4)

add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "Elliott Wave in FX: Wave 3 is typically the longest and strongest. Wave 4 cannot overlap Wave 1 territory. Wave 5 often shows momentum divergence. Corrective waves (A-B-C) retrace 38.2%-61.8% of impulse.", 4)

add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "Fibonacci in FX: 38.2% and 61.8% retracements are most common in trending markets. 76.4% retracement is last defense before trend invalidation. Extensions at 161.8% and 261.8% project wave targets.", 4)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeGreed", "ExtremeFear"],
    "Google Trends as sentiment proxy: Search volume for 'weak dollar' peaked at DXY major bottoms (Dec 2004, late 2006). News reference volume peaks at turning points. Combine with price action for timing.", 3)

add("saettele-sentiment-forex", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Why most traders lose: They extrapolate recent trends. Media is always wrong at turns because reporters reflect public sentiment. At tops, hope dominates. At bottoms, fear dominates. The crowd is always wrong at extremes.", 5)

add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "Slow stochastics: Use %K and %D crossovers in trending markets for entry timing. In range-bound markets, use overbought (>80) and oversold (<20) levels. Adjust approach based on ADX reading.", 3)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeGreed"],
    "Open interest in currency futures: Rising open interest + rising price = trend strengthening. Rising open interest + falling price = new shorts entering, trend continuing. Falling OI at extremes signals trend exhaustion.", 4)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeFear", "ExtremeGreed"],
    "SSI (Speculative Sentiment Index): When retail traders are heavily long, price tends to fall. When heavily short, price tends to rise. SSI is a contrarian indicator - trade against the crowd.", 5)

add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "Support and resistance from sentiment: Price often respects levels where previous sentiment extremes occurred. Old highs where euphoria peaked become resistance. Old lows where panic occurred become support.", 3)

add("saettele-sentiment-forex", "Sentiment", ["Trending", "Ranging"],
    "Combining sentiment with technicals: Only take trades when sentiment extreme aligns with key technical level. Magazine cover + Fibonacci 61.8% retracement + COT extreme = highest conviction setup.", 4)

add("saettele-sentiment-forex", "Psychology", ["ExtremeFear"],
    "Fear is stronger than greed: Bearish moves are faster and sharper than bullish moves. This is why tops form slowly (distribution) but bottoms form quickly (capitulation). Position sizing should reflect this asymmetry.", 4)

# =========================================================================
# BOOK 5: Ed Ponsi - Forex Patterns and Probabilities
# =========================================================================

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Trending"],
    "EMA bounce strategy: Use 20 EMA on hourly chart. In uptrend, buy when price pulls back to 20 EMA and bounces with bullish candle. Stop below recent swing low. Works best when ADX > 25 confirming trend strength.", 4)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Ranging"],
    "FX Power Play: Identify pairs making new highs/lows while other pairs fail to confirm. Divergence between correlated pairs signals potential reversal. Enter when divergence is confirmed by price action.", 4)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Ranging"],
    "Daily Double Trap: On ranging days where price reverses twice, enter on the second reversal. If pair makes new daily low then reverses above opening, go long. Tight stop below the false breakdown level.", 4)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Trending"],
    "Moving Average Ribbon: Use 10, 20, 50 EMA. When all three align (10>20>50 for uptrend), trend is strong. Enter on pullbacks to 20 EMA. When ribbon flattens, trend is ending - tighten stops or exit.", 4)

add("ponsi-patterns-probabilities", "RiskManagement", ["Trending", "Ranging"],
    "Scaling out: Exit 1/3 position at 1x risk, 1/3 at 2x risk, final 1/3 with trailing stop. This locks in profits while allowing runners. Reduces emotional pressure of all-or-nothing exits.", 4)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Ranging"],
    "Boomerang strategy: After news spike, wait for price to retrace 50-61.8% of the spike. Enter in direction of original spike. Stop below 76.4% retracement. News provides direction, pullback provides entry.", 4)

add("ponsi-patterns-probabilities", "Execution", ["SessionOpen"],
    "Asian session range break: Monitor EUR/USD range during Tokyo session (typically 30-50 pips). Enter on London open breakout of Asian range. Stop at opposite end of Asian range. Best on days with no major US data.", 4)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Trending"],
    "Trend line breaks with volume confirmation: Break of trend line alone is insufficient. Wait for candle close beyond trend line, then retest from other side. Enter on successful retest. Stop beyond the break point.", 3)

add("ponsi-patterns-probabilities", "RiskManagement", ["Trending", "Ranging"],
    "Position sizing formula: Position size = (Account Risk $) / (Stop Loss pips x pip value). Always calculate before entry. Never exceed 2% account risk. Adjust size inversely with stop distance.", 5)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Ranging"],
    "Bollinger Band squeeze: When bandwidth contracts to multi-month low, explosive move is imminent. Direction uncertain - use straddle approach or wait for breakout confirmation. The tighter the squeeze, the bigger the move.", 4)

add("ponsi-patterns-probabilities", "MacroAnalysis", ["BreakingNews"],
    "NFP trading: Don't trade 30 min before or 15 min after NFP release. Wait for dust to settle, then trade the established direction. Initial spike is often reversed. The secondary move is the real move.", 5)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Trending"],
    "Price confirmation: Never enter based on single signal. Require at least 2 confirming factors: trend direction, support/resistance level, candlestick pattern, indicator signal, or time-of-day edge.", 4)

# =========================================================================
# BOOK 6: Greg Michalowski - Attacking Currency Trends
# =========================================================================

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "100 bar MA on 4H and daily charts defines trend. Price above = look for buys only. Price below = look for sells only. This simple filter eliminates counter-trend trades which have lower win rates.", 5)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "Trend line tool: Connect 2-3 swing lows in uptrend (or highs in downtrend). Break of trend line signals potential trend change but NOT reversal. Wait for break of last swing low/high for confirmation.", 4)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "38.2%-61.8% sweet spot: In strong trends, pullbacks typically hold between 38.2% and 61.8% Fibonacci retracement. Enter in this zone with stop below 76.4%. If price breaks 76.4%, trend structure is likely broken.", 5)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "Swing trading with 4H charts: Identify swing highs and lows on 4H. Enter on break of swing high in uptrend (or swing low in downtrend). Use 1H for fine-tuning entries. Trail stops using swing points.", 4)

add("michalowski-attacking-trends", "RiskManagement", ["Trending"],
    "Risk management for trends: Start with 0.5% risk per trade. Add to winners up to 1% total risk on best setups. Trail stop to breakeven at 1x risk. Move stop to lock in 1x risk profit at 2x target.", 4)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "Key technical levels: Major swing points, round numbers, and Fibonacci levels create 'barriers' where institutional orders cluster. Price accelerates through these levels or bounces sharply. No middle ground.", 4)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Ranging"],
    "Consolidation identification: When price makes lower highs and higher lows (contracting range), breakout is imminent. Measure the range height for target projection. Enter on break with stop at opposite side of range.", 4)

add("michalowski-attacking-trends", "Execution", ["SessionOpen"],
    "Session-based trend starts: Many trends begin at London open or US open. Monitor first 2 hours of each session for directional bias. If trend begins at London open, it often continues through NY overlap.", 3)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "Moving average slope: Increasing MA slope = accelerating trend. Decreasing slope = decelerating. Flat MA = ranging. Use slope direction change as early warning of trend phase change.", 3)

add("michalowski-attacking-trends", "RiskManagement", ["Trending"],
    "Maximum adverse excursion (MAE): Study how far trades go against you before hitting target. Use MAE data to set stops that are wide enough to avoid noise but tight enough for good R:R.", 3)

# =========================================================================
# BOOK 7: Nekritin & Peters - Naked Forex
# =========================================================================

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Support/resistance zones (not lines): Zones are areas where price repeatedly reverses. Use line charts to find zones easily. Old zones work better than new ones - market has memory. Zones are 'market scars'.", 5)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "The Last Kiss: After price breaks through a zone, wait for it to retest the zone from the other side. Enter on the retest (the 'last kiss'). Stop beyond the zone. The retest confirms the breakout is real.", 5)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "Big Shadow: A candle whose range completely engulfs the prior candle. Bullish Big Shadow at support zone = buy signal. Bearish Big Shadow at resistance zone = sell signal. Enter on break of shadow, stop at opposite end.", 5)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Ranging"],
    "Kangaroo Tail: Long-wick candle at zone with small body. Wick shows rejection of price level. The longer the wick relative to body, stronger the signal. Enter on break of tail body, stop at tail tip.", 5)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "Wammies and Moolahs: Double bottom/top patterns at zones. Wammie = W-shaped double bottom at support. Moolah = M-shaped double top at resistance. Enter on break of neckline, stop beyond the zone.", 4)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "The Big Belt: Large engulfing candle at zone that 'belts' the prior candle. Stronger than Big Shadow because it shows complete dominance. Enter on break, stop at belt candle midpoint. High success rate at key zones.", 4)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "The Trendy Kangaroo: Kangaroo tail pattern that occurs WITH the trend (pullback entry). More powerful than counter-trend kangaroo tails. Wait for pullback to zone, look for kangaroo tail forming at zone.", 4)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Zone identification on higher timeframes (weekly/daily), trade setups on lower timeframes (4H/1H). Use line chart to find zones, switch to candlestick for entry patterns. Multi-TF confluence increases probability.", 5)

add("nekritin-naked-forex", "Backtesting", ["Trending", "Ranging"],
    "Manual backtesting > automated: Manual testing builds expertise and pattern recognition. Keep sessions under 2 hours. Test at least 200 trades before going live. Track win rate, avg win/loss, max drawdown.", 5)

add("nekritin-naked-forex", "RiskManagement", ["Trending", "Ranging"],
    "Risk 1-2% per trade maximum. Place stop beyond the zone (not at arbitrary pip distance). If zone is too wide for acceptable risk, skip the trade. Never widen a stop after entry.", 5)

add("nekritin-naked-forex", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "The Forex Cycle: Excitement > obsession > frustration > depression > acceptance. Recognize which stage you're in. Most traders cycle through these stages repeatedly. Self-awareness breaks the cycle.", 4)

add("nekritin-naked-forex", "Psychology", ["Trending", "Ranging"],
    "Trading system is less important than beliefs about risk. System accounts for 10% of success. Psychology and risk management account for 90%. Simple systems are robust; complex systems break.", 5)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "No indicators needed: All indicators derive from price with a lag. Price action at zones provides the same information faster. Removing indicators forces you to read price directly - superior long-term.", 4)

add("nekritin-naked-forex", "Execution", ["Trending", "Ranging"],
    "Time of day matters: Asian session is quiet (zone building). London open is active (breakouts). NY overlap is most volatile (trend moves). Align your setups with session characteristics.", 4)

add("nekritin-naked-forex", "RiskManagement", ["Trending", "Ranging"],
    "Multiple positions approach: Enter 2-3 positions. Close first at 1:1 R:R (covers risk), close second at 2:1, trail third position. This ensures profit on most trades while allowing runners.", 4)

# =========================================================================
# BOOK 8: Kathy Lien - The Little Book of Currency Trading
# =========================================================================

add("kathy-lien-little-book", "MacroAnalysis", ["Trending"],
    "Carry trade simplified: Buy high-yield currency, sell low-yield. Earn daily interest (rollover). Works in stable/uptrending markets. Avoid when VIX rising or equity markets falling sharply.", 5)

add("kathy-lien-little-book", "MacroAnalysis", ["FomcDate"],
    "Central bank meetings are the #1 catalyst for currency moves. Rate decisions cause immediate spikes. Post-meeting statements provide forward guidance. Hawkish shift = currency bullish. Dovish shift = bearish.", 5)

add("kathy-lien-little-book", "RiskManagement", ["HighVolatility"],
    "Leverage is double-edged: 50:1 leverage means 2% adverse move wipes out 100% of account. Use leverage conservatively - 10:1 maximum for beginners. Professional traders rarely exceed 5:1 effective leverage.", 5)

add("kathy-lien-little-book", "Sentiment", ["Trending"],
    "Retail positioning as contrarian signal: When 80%+ of retail traders are long, price tends to fall. When 80%+ are short, price tends to rise. Broker-provided sentiment data is a free edge.", 5)

add("kathy-lien-little-book", "TechnicalAnalysis", ["Trending"],
    "Trend following is the highest probability strategy in FX. Currency trends persist longer than in other markets due to interest rate differentials and macroeconomic momentum.", 4)

add("kathy-lien-little-book", "MacroAnalysis", ["BreakingNews"],
    "Don't trade the news, trade the aftermath. Initial spike is often reversed. Wait 15-30 min for dust to settle, then trade the direction that holds. Use the pre-news range for stop placement.", 4)

add("kathy-lien-little-book", "RiskManagement", ["Trending", "Ranging"],
    "Always use stop losses. Never trade without one. Place stops at technical levels, not arbitrary pip distances. If you can't define your stop before entry, you don't have a trade - you have a gamble.", 5)

add("kathy-lien-little-book", "Execution", ["SessionOpen", "SessionClose"],
    "Best times to trade: US-European overlap (8AM-12PM EST) for volatility. Avoid Friday afternoon (weekend gap risk) and Sunday evening (thin liquidity, erratic moves).", 4)

add("kathy-lien-little-book", "TechnicalAnalysis", ["Trending"],
    "Double zeros and option barriers: Large option barriers sit at round numbers (1.3000, 110.00). Price is 'pulled' toward these levels on option expiry days (usually NY cut at 10AM EST).", 4)

add("kathy-lien-little-book", "MacroAnalysis", ["Trending"],
    "Currency pair selection: Trade the strongest vs weakest currency for best trending behavior. If USD is strongest and JPY is weakest, buy USD/JPY. Cross pairs (EUR/GBP, AUD/NZD) have lower liquidity.", 4)

add("kathy-lien-little-book", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "FOMO (fear of missing out) causes impulsive entries after big moves. If you missed the move, wait for pullback. There's always another setup. Patience is the most profitable skill in FX trading.", 4)

add("kathy-lien-little-book", "RiskManagement", ["Trending", "Ranging"],
    "Reward-to-risk minimum 2:1. With 2:1 R:R, you only need 34% win rate to be profitable. With 3:1 R:R, you only need 25% win rate. Focus on finding high R:R setups, not high win rate.", 5)

# =========================================================================
# BOOK 9: Cliff Wachtel - The Sensible Guide to Forex
# =========================================================================

add("wachtel-sensible-forex", "RiskManagement", ["Trending", "Ranging"],
    "Risk budget approach: Allocate max 1-3% of total capital per trade. Calculate pip value before entry. Use micro lots (0.01) when learning. Scale up only after 3+ months of consistent profits.", 5)

add("wachtel-sensible-forex", "Execution", ["Trending", "Ranging"],
    "Demo trade for minimum 3 months before live. Then start with minimum position size. Treat demo money as real - habits formed in demo carry to live trading.", 4)

add("wachtel-sensible-forex", "MacroAnalysis", ["Trending"],
    "Fundamental analysis hierarchy: 1) Interest rate expectations (most important) 2) Economic growth differentials 3) Trade and capital flows 4) Political stability 5) Market sentiment. Rate expectations dominate.", 4)

add("wachtel-sensible-forex", "RiskManagement", ["HighVolatility"],
    "Hedging with options: Buy protective puts on currency pairs you're long. Cost is premium paid, but limits downside. Most effective ahead of known risk events (elections, central bank meetings).", 3)

add("wachtel-sensible-forex", "MacroAnalysis", ["Trending"],
    "Safe haven currencies: JPY and CHF appreciate during global risk aversion. USD benefits from flight to safety. AUD, NZD, and EM currencies sold off. Monitor VIX, credit spreads, and equity indices for risk signals.", 4)

add("wachtel-sensible-forex", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Multi-timeframe confirmation: Weekly for trend, daily for direction, 4H for setup, 1H for entry. All four must align for highest conviction trades. Skip trades where lower TF contradicts higher TF.", 4)

add("wachtel-sensible-forex", "RiskManagement", ["HighVolatility"],
    "Correlation risk: Simultaneously long EUR/USD and short USD/CHF is essentially doubling the same trade. Check correlation matrix before adding positions. Avoid correlated exposure > 5% of account.", 4)

add("wachtel-sensible-forex", "MacroAnalysis", ["Trending"],
    "Emerging market currencies offer highest yields but carry highest risk. Political instability, capital controls, and sudden devaluations are real risks. Stick to major pairs when starting.", 3)

add("wachtel-sensible-forex", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Revenge trading: After a loss, the urge to immediately re-enter is strongest. This almost always leads to more losses. Implement a 'cooling off' rule: wait 1 hour minimum after any loss before next trade.", 4)

add("wachtel-sensible-forex", "Execution", ["SessionOpen"],
    "Broker selection criteria: Regulated (NFA/FCA), competitive spreads, reliable execution, negative balance protection, segregated client funds. Avoid unregulated offshore brokers despite higher leverage offered.", 4)

# =========================================================================
# BOOK 10: James Chen - Essentials of Foreign Exchange Trading
# =========================================================================

add("chen-essentials-fx", "TechnicalAnalysis", ["Trending"],
    "Head and shoulders in FX: Most reliable reversal pattern. Neckline break confirms. Measure head-to-neckline for target projection. In FX, shoulders are often asymmetric - that's normal.", 4)

add("chen-essentials-fx", "TechnicalAnalysis", ["Trending"],
    "Triangle breakouts: Ascending triangle = bullish bias (flat top, rising bottom). Descending = bearish. Symmetrical = neutral (break either direction). Enter on break with stop at last swing point.", 4)

add("chen-essentials-fx", "TechnicalAnalysis", ["Ranging"],
    "Double top/bottom: Second peak/trough may exceed first slightly (false breakout). Enter on neckline break. Target = height of pattern projected from neckline. More reliable on daily+ timeframes.", 3)

add("chen-essentials-fx", "RiskManagement", ["Trending", "Ranging"],
    "The 2% rule is absolute: Never risk more than 2% of account on single trade. If account is $10,000, max risk is $200. Calculate position size from this number and your stop distance.", 5)

add("chen-essentials-fx", "MacroAnalysis", ["FomcDate"],
    "Interest rate expectations drive FX more than actual rates. Market prices in expected moves ahead of time. A rate hike that's fully priced in may cause the currency to sell off (buy the rumor, sell the fact).", 5)

add("chen-essentials-fx", "TechnicalAnalysis", ["Trending"],
    "Candlestick patterns in FX: Engulfing patterns at key levels are strongest reversal signals. Morning/evening star patterns work well on daily charts. Pin bars (long wicks) show rejection of price level.", 3)

add("chen-essentials-fx", "Execution", ["SessionOpen", "SessionClose"],
    "Session overlap strategy: Trade breakouts during London-NY overlap when liquidity peaks. Asian session is for range identification. Set pending orders above/below Asian range for London open.", 4)

add("chen-essentials-fx", "TechnicalAnalysis", ["Trending"],
    "MACD signal line crossovers: Buy when MACD crosses above signal line in uptrend. Sell when crosses below in downtrend. Use histogram for momentum assessment. Divergence between MACD and price warns of reversal.", 3)

add("chen-essentials-fx", "MacroAnalysis", ["Trending"],
    "GDP, employment, inflation, and trade balance are the four pillars of fundamental analysis. Changes in expectations matter more than absolute numbers. Watch for revisions to prior data.", 3)

add("chen-essentials-fx", "RiskManagement", ["HighVolatility"],
    "Volatility-adjusted position sizing: Use ATR to measure volatility. Wider stops for volatile pairs = smaller position. Tighter stops for quiet pairs = larger position. Keeps dollar risk constant.", 4)

# =========================================================================
# BOOK 11: Courtney Smith - How to Make a Living Trading FX
# =========================================================================

add("smith-living-trading-fx", "TechnicalAnalysis", ["Trending"],
    "The 'Oops' trade: When price gaps at open and immediately reverses, enter in reversal direction. Gap + reversal = trapped traders. Tight stop beyond gap. Works on daily charts for swing trades.", 4)

add("smith-living-trading-fx", "TechnicalAnalysis", ["Trending"],
    "Power Momentum Strategy: Buy when price closes above 20-period EMA AND RSI is above 50 AND MACD histogram is positive. All three must align. Exit when any one condition fails.", 4)

add("smith-living-trading-fx", "RiskManagement", ["Trending", "Ranging"],
    "Maximum portfolio heat: Total open risk across all positions should not exceed 6-8% of account. Individual position risk 1-2%. This prevents catastrophic drawdown from correlated losing trades.", 5)

add("smith-living-trading-fx", "TechnicalAnalysis", ["Trending"],
    "Channel breakout: When price breaks above a 20-day channel high, go long. When breaks below 20-day channel low, go short. Use ATR for stop placement (2x ATR). Simple but effective trend-following system.", 4)

add("smith-living-trading-fx", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "The 10-trade rule: Evaluate your system over minimum 10 trades before making changes. Changing rules after 2-3 losses is how traders destroy working systems. Statistics require sample size.", 5)

add("smith-living-trading-fx", "Execution", ["SessionOpen"],
    "Opening range breakout: First 30 minutes of London session defines the range. Buy above range high, sell below range low. Stop at opposite end. Works best on volatile pairs (GBP/JPY, GBP/CHF).", 4)

add("smith-living-trading-fx", "RiskManagement", ["Trending"],
    "Pyramid into winners: Start with half position. Add quarter position at 1x risk profit. Add another quarter at 2x risk profit. Stop all at 1x risk below current price. Never pyramid into losers.", 4)

add("smith-living-trading-fx", "MacroAnalysis", ["Trending"],
    "Carry trade management: Collect interest daily but monitor risk sentiment closely. Exit immediately when VIX spikes above 25 or equity markets drop >2% in a day. Carry can't compensate for capital loss.", 4)

add("smith-living-trading-fx", "TechnicalAnalysis", ["Ranging"],
    "RSI range trading: When RSI oscillates between 30-70 for 10+ bars, market is range-bound. Buy RSI 30, sell RSI 70. Exit at opposite extreme. Stop beyond range boundaries.", 3)

add("smith-living-trading-fx", "Psychology", ["Trending", "Ranging"],
    "Trading plan must be written and followed. Entry rules, exit rules, risk rules, and daily routine all documented. Review weekly. Deviation from plan = gambling, not trading.", 5)

# =========================================================================
# BOOK 12: Callum Henderson - Currency Strategy
# =========================================================================

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "PPP (Purchasing Power Parity) is the long-term anchor. Currencies can deviate from PPP for years due to capital flows, but always revert. OECD publishes PPP estimates. Use as directional bias for multi-year positions.", 4)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "BEER (Behavioral Equilibrium Exchange Rate): Combines productivity differentials, terms of trade, and net foreign assets. More practical than pure PPP for identifying medium-term currency misalignment.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Carry trade as systematic strategy: Equal-weight long high-yielders, short low-yielders. Returns are positive but with significant negative skewness (occasional large losses). Sharpe ratio ~0.5 historically.", 4)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Terms of trade effect: Rising commodity prices improve terms of trade for commodity exporters (AUD, CAD, NZD), strengthening their currencies. Monitor CRB index as leading indicator.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["FomcDate"],
    "Central bank credibility: Currencies of central banks with strong inflation-fighting credibility (ECB, SNB) trade at premium. Currencies of less credible central banks require higher yields to attract capital.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Flow analysis: Track cross-border M&A flows, portfolio flows, and official flows. Large M&A deals create temporary but significant currency demand. Japanese year-end repatriation (March) affects JPY.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Current account sustainability: Deficits >4% of GDP are unsustainable long-term. Country must attract equivalent capital inflows or currency adjusts. US current account deficit was key USD bear argument 2002-2007.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Productivity growth differential: Country with faster productivity growth sees real exchange rate appreciation (Balassa-Samuelson effect). Asian currencies with strong productivity growth are structurally undervalued on PPP basis.", 3)

add("henderson-currency-strategy", "RiskManagement", ["HighVolatility"],
    "Options-implied volatility as risk gauge: When 1-month implied vol > 3-month (inverted term structure), market expects imminent crisis. Reduce exposure. Normal term structure (upward sloping) = stable conditions.", 4)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "FX reserves accumulation: Central bank reserve diversification flows can move markets. China's shift from USD to EUR reserves, or BOJ intervention, creates structural flows that override technical levels.", 3)

# =========================================================================
# BOOK 13: Mario Singh - 17 Proven Currency Trading Strategies
# =========================================================================

add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "Moving average crossover: 5 EMA crosses above 20 EMA = buy signal. Cross below = sell. Filter with 200 SMA direction (only buy above 200, sell below). Simple but effective trending strategy.", 4)

add("singh-17-strategies", "TechnicalAnalysis", ["Ranging"],
    "Bollinger Band reversal: When price touches lower band and RSI < 30, look for buy signal on next candle. When price touches upper band and RSI > 70, look for sell. Works in range-bound markets only.", 4)

add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "MACD histogram strategy: When histogram turns from negative to positive, buy. When turns from positive to negative, sell. Stronger signal when histogram crosses zero line.", 3)

add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "Ichimoku Cloud: Price above cloud = bullish, below = bearish. Cloud twist = trend change. TK cross (Tenkan crosses Kijun) generates signals. Thick cloud = strong support/resistance.", 3)

add("singh-17-strategies", "RiskManagement", ["Trending", "Ranging"],
    "Risk per trade calculator: Risk% x Account Balance / (Stop Loss in Pips x Pip Value) = Lot Size. Example: 2% x $10,000 / (50 pips x $10/pip) = 0.4 lots. Always calculate before entry.", 5)

add("singh-17-strategies", "MacroAnalysis", ["FomcDate"],
    "Interest rate differential strategy: Long the currency pair where rate differential is widening (e.g., Fed hiking, ECB cutting = short EUR/USD). Monitor central bank dot plots and forward guidance.", 4)

add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "Fibonacci extension targets: Use 127.2% and 161.8% extensions for profit targets in trending markets. Project from the start of the move through the first retracement point.", 3)

add("singh-17-strategies", "Execution", ["BreakingNews"],
    "News straddle strategy: Place buy stop 20 pips above and sell stop 20 pips below current price before major news. One gets triggered. Cancel the other. Use tight stop on triggered order.", 3)

add("singh-17-strategies", "TechnicalAnalysis", ["Ranging"],
    "Stochastic oscillator range trading: Buy when %K crosses above %D below 20. Sell when %K crosses below %D above 80. Use only in range-bound markets (ADX < 25).", 3)

add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "Parabolic SAR trailing stop: Use Parabolic SAR as trailing stop in trending markets. When dots flip from below to above price, exit long. When flip from above to below, exit short.", 3)

add("singh-17-strategies", "MacroAnalysis", ["Trending"],
    "Economic calendar strategy: Trade the trend, avoid data releases. Close positions 30 min before high-impact news. Re-enter after volatility settles if trend remains intact.", 4)

add("singh-17-strategies", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Win rate is overrated. A 40% win rate with 3:1 R:R is more profitable than 70% win rate with 1:1 R:R. Focus on R:R, not on being right. Being profitable and being right are different things.", 5)

# =========================================================================
# BOOK 14: Forex Trading Basics & Secrets Volume 3.0
# =========================================================================

add("forex-basics-secrets", "Execution", ["SessionOpen"],
    "London open strategy: Asian session range for EUR/USD is typically 30-50 pips. Place buy stop above Asian high + 10 pips, sell stop below Asian low - 10 pips. Cancel unfilled order when other triggers.", 4)

add("forex-basics-secrets", "TechnicalAnalysis", ["Trending"],
    "Support becomes resistance and vice versa: When a key level breaks, it flips roles. Old support becomes new resistance. Enter on retest of broken level. Stop beyond the level.", 4)

add("forex-basics-secrets", "RiskManagement", ["Trending", "Ranging"],
    "The 1:3 rule: Risk 1% of account, target 3% profit per trade. Even with 50% win rate, this produces 10% monthly return. Don't chase high win rates - chase high reward:risk.", 5)

add("forex-basics-secrets", "TechnicalAnalysis", ["Trending"],
    "Pin bar strategy: Long lower wick at support = bullish (buyers rejected lower prices). Long upper wick at resistance = bearish (sellers rejected higher prices). Enter on break of pin bar body, stop at wick tip.", 4)

add("forex-basics-secrets", "Execution", ["SessionOpen", "SessionClose"],
    "Best trading hours: London-NY overlap (8AM-12PM EST) for majors. Asian session for JPY/AUD pairs only. Avoid Friday afternoon (weekend gaps) and Monday Asian open (thin liquidity).", 4)

add("forex-basics-secrets", "TechnicalAnalysis", ["Trending"],
    "Higher highs and higher lows define uptrend. Lower highs and lower lows define downtrend. Enter on pullback to higher low (buy) or lower high (sell). Simple but most reliable trend definition.", 5)

add("forex-basics-secrets", "RiskManagement", ["HighVolatility"],
    "Never move stop loss further from entry. Only move it to lock in profits (trailing stop). Moving stops wider is how small losses become account-destroying losses.", 5)

add("forex-basics-secrets", "TechnicalAnalysis", ["Ranging"],
    "Range identification: If price has bounced off same support and resistance levels 3+ times, it's a range. Buy support, sell resistance. Stop just beyond range. Target opposite end.", 4)

add("forex-basics-secrets", "MacroAnalysis", ["BreakingNews"],
    "High-impact news causes 50-200 pip moves in seconds. Spreads widen dramatically. Slippage is common. Either trade news with proper strategy or stay flat. Half-measures (hoping) don't work.", 4)

add("forex-basics-secrets", "Psychology", ["Trending", "Ranging"],
    "Journal every trade: Entry reason, exit reason, emotions during trade, lessons learned. Review weekly. Patterns emerge - you'll discover your personal weaknesses and strengths.", 4)

# =========================================================================
# BOOK 15: Anna Coulling - Forex for Beginners
# =========================================================================

add("coulling-forex-beginners", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Volume spread analysis (VSA): Wide spread candle on high volume = professional activity. Narrow spread on high volume = accumulation/distribution. Compare spread and volume to identify smart money.", 4)

add("coulling-forex-beginners", "TechnicalAnalysis", ["Trending"],
    "Wyckoff method in FX: Accumulation (smart money buying) > Markup (trend up) > Distribution (smart money selling) > Markdown (trend down). Identify phase by price action and volume patterns.", 4)

add("coulling-forex-beginners", "TechnicalAnalysis", ["Trending"],
    "Three timeframes: Monthly/weekly for major trend, daily for swing trades, 4H/1H for entries. Never trade against the weekly trend. Use daily for timing, not direction.", 4)

add("coulling-forex-beginners", "RiskManagement", ["Trending", "Ranging"],
    "Start with one pair only. Learn its personality, average range, key levels, and behavior during different sessions. Specialization beats diversification for beginners.", 5)

add("coulling-forex-beginners", "Execution", ["SessionOpen"],
    "Candlestick analysis by session: Asian session candles form the 'setup' (consolidation). London session candles provide the 'trigger' (breakout). NY session provides the 'follow-through'.", 4)

add("coulling-forex-beginners", "TechnicalAnalysis", ["Trending"],
    "Price action hierarchy: 1) Market structure (trend/range) 2) Key levels (S/R zones) 3) Candlestick patterns (entry signals). Always analyze in this order. Never skip to candlestick patterns without context.", 5)

add("coulling-forex-beginners", "MacroAnalysis", ["Trending"],
    "Forex is driven by expectations, not reality. If market expects 200K jobs and gets 150K, USD falls. If market expects 100K and gets 150K, USD rises. Same number, opposite reaction.", 5)

add("coulling-forex-beginners", "TechnicalAnalysis", ["Ranging"],
    "Pivot points: Calculate daily/weekly pivots. Price above pivot = bullish bias, below = bearish. R1/R2 and S1/S2 provide natural profit targets and entry levels.", 3)

add("coulling-forex-beginners", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Demo account discipline: Treat demo capital as real money. If you can't follow rules in demo, you won't in live. Demo success for 3 months minimum before going live.", 4)

add("coulling-forex-beginners", "RiskManagement", ["Trending", "Ranging"],
    "Leverage reality: 100:1 leverage on $1000 account controls $100,000. A 1% move = $1000 gain or loss (100% of account). Use leverage as tool, not weapon. Most professionals use 5-10x effective leverage.", 5)

# =========================================================================
# BOOK 16: Kathleen Brooks & Brian Dolan - Currency Trading for Dummies
# =========================================================================

add("brooks-dolan-dummies", "TechnicalAnalysis", ["Trending"],
    "Trend identification: Use 200 SMA on daily chart. Price above = uptrend, below = downtrend. 50 SMA for intermediate trend. When 50 crosses 200 (golden/death cross), major trend change.", 5)

add("brooks-dolan-dummies", "TechnicalAnalysis", ["Trending"],
    "Fibonacci retracement: Draw from significant swing low to high (uptrend) or high to low (downtrend). 38.2%, 50%, and 61.8% are key levels. Price often bounces at 50% in strong trends.", 4)

add("brooks-dolan-dummies", "MacroAnalysis", ["FomcDate"],
    "Central bank watching: Divergent monetary policies create strongest trends. Monitor rate expectations via overnight index swaps (OIS). Forward guidance often more important than actual rate decisions.", 4)

add("brooks-dolan-dummies", "RiskManagement", ["Trending", "Ranging"],
    "Order types: Market orders for immediate execution. Limit orders for better price (entries). Stop orders for breakout entries and stop losses. Stop-limit for precise control. Know when to use each.", 4)

add("brooks-dolan-dummies", "MacroAnalysis", ["Trending"],
    "Economic data interpretation: Nonfarm Payrolls = king of data. CPI = inflation gauge. GDP = growth measure. PMI = leading indicator. Trade balance = flow data. Each impacts currencies differently.", 4)

add("brooks-dolan-dummies", "Execution", ["SessionOpen", "SessionClose"],
    "24-hour market: Sydney opens 5PM EST, Tokyo 7PM, London 2AM, NY 8AM. Three main sessions with different characteristics. Overlap periods have highest volume and volatility.", 4)

add("brooks-dolan-dummies", "TechnicalAnalysis", ["Trending"],
    "Chart patterns in FX: Triangles, flags, pennants, channels. Measure pattern height for target projection. Breakout volume confirmation important in futures (FX lacks true volume).", 3)

add("brooks-dolan-dummies", "RiskManagement", ["HighVolatility"],
    "Correlation management: EUR/USD and GBP/USD are 80-90% correlated. Going long both is doubling exposure. Use correlation matrix to ensure portfolio diversification.", 4)

add("brooks-dolan-dummies", "MacroAnalysis", ["Trending"],
    "Carry trade: Buy AUD/JPY or NZD/JPY to earn interest differential. Works when risk appetite is strong and volatility is low. Unwinds sharply during risk-off events.", 4)

add("brooks-dolan-dummies", "TechnicalAnalysis", ["Ranging"],
    "Range trading rules: Identify clear support and resistance. Buy at support with stop below. Sell at resistance with stop above. Target opposite end of range. Only trade ranges when ADX < 25.", 4)

add("brooks-dolan-dummies", "MacroAnalysis", ["BreakingNews"],
    "News trading caution: Spreads widen 5-10x during major releases. Slippage common. Use guaranteed stops if available. Better to wait for post-news clarity than trade the initial spike.", 4)

add("brooks-dolan-dummies", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Common mistakes: Overtrading, no stop losses, risking too much, revenge trading, moving stops, ignoring the trend. Every one of these is a discipline problem, not a knowledge problem.", 5)

# =========================================================================
# BOOK 17: Jared Martinez - 10 Essentials of Forex Trading
# =========================================================================

add("martinez-10-essentials", "TechnicalAnalysis", ["Trending"],
    "The Forex market pattern: Markets alternate between trend and consolidation. Learn to identify which phase you're in. Trend phases: use moving averages and breakout strategies. Range phases: use oscillators.", 5)

add("martinez-10-essentials", "TechnicalAnalysis", ["Trending"],
    "Fibonacci mastery: The market respects Fibonacci levels because everyone watches them. Self-fulfilling prophecy. 38.2% shallow pullback in strong trend, 61.8% normal pullback, 76.4% deep pullback (weakening trend).", 4)

add("martinez-10-essentials", "TechnicalAnalysis", ["Trending"],
    "Candlestick patterns: Hammer/hanging man, engulfing patterns, doji at key levels. Single candle patterns have lower reliability than multi-candle patterns. Combine with S/R for confirmation.", 3)

add("martinez-10-essentials", "RiskManagement", ["Trending", "Ranging"],
    "The 90% rule: 90% of traders lose 90% of their money in 90 days. The difference between winners and losers is not knowledge but discipline and risk management. Protect capital first, profits second.", 5)

add("martinez-10-essentials", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Fear and greed cycle: Fear causes early exit of winners. Greed causes holding losers too long. The solution is mechanical rules: fixed stops, predetermined targets, position sizing formula.", 5)

add("martinez-10-essentials", "TechnicalAnalysis", ["Trending"],
    "Moving average as dynamic support/resistance: In uptrend, 20 EMA acts as dynamic support. Price pulls back to it and bounces. In downtrend, 20 EMA acts as dynamic resistance. Trade bounces off MA.", 4)

add("martinez-10-essentials", "Execution", ["SessionOpen"],
    "Session characteristics: Tokyo = quiet ranging, London = trend initiation, NY = trend continuation or reversal. Trade strategies that match session behavior.", 3)

add("martinez-10-essentials", "RiskManagement", ["Trending", "Ranging"],
    "Position sizing example: $10,000 account, 2% risk = $200 max loss. If stop is 50 pips, position size = $200/50 = $4/pip = 0.4 mini lots. This is your MAX position regardless of confidence level.", 5)

add("martinez-10-essentials", "TechnicalAnalysis", ["Ranging"],
    "Inside bar strategy: Small candle contained within prior candle's range. Represents compression before expansion. Enter on break of mother candle. Stop at opposite end. Works on all timeframes.", 3)

add("martinez-10-essentials", "Psychology", ["Trending", "Ranging"],
    "Trading is a business, not gambling. Create a business plan with: strategy rules, risk parameters, daily routine, profit targets, maximum drawdown limits. Review monthly.", 5)

# =========================================================================
# BOOK 18: James Dicks - Forex Trading Secrets
# =========================================================================

add("dicks-forex-secrets", "MacroAnalysis", ["Trending"],
    "Carry trade mechanics: Borrow JPY at 0.5%, invest in AUD at 7.25%. Earn 6.75% annual differential. Works until risk aversion triggers JPY strength and carry unwinds violently.", 5)

add("dicks-forex-secrets", "TechnicalAnalysis", ["Trending"],
    "Chart patterns: Head and shoulders, double tops/bottoms, triangles, wedges, flags. Most reliable on daily charts. Measure height for target. Breakout must be confirmed by close beyond pattern boundary.", 3)

add("dicks-forex-secrets", "TechnicalAnalysis", ["Trending"],
    "Harmonic patterns: Gartley, butterfly, crab, bat patterns use specific Fibonacci ratios for each leg. AB=CD pattern is simplest. More advanced but high accuracy when correctly identified.", 3)

add("dicks-forex-secrets", "TechnicalAnalysis", ["Ranging"],
    "Breakout strategy: Identify consolidation (range). Place buy stop above range, sell stop below. One triggers, cancel other. Stop at opposite end. Target = range height projected from breakout point.", 4)

add("dicks-forex-secrets", "RiskManagement", ["HighVolatility"],
    "ATR-based stops: Use 1.5-2x ATR(14) for stop distance. This adapts to current volatility. In high volatility, stops are wider (smaller position). In low volatility, stops are tighter (larger position).", 5)

add("dicks-forex-secrets", "Execution", ["SessionOpen", "SessionClose"],
    "Session trading plan: Before London open - identify key levels and bias. London session - trade breakouts of Asian range. NY overlap - trade trend continuation. Afternoon NY - reduce exposure.", 4)

add("dicks-forex-secrets", "TechnicalAnalysis", ["Trending"],
    "Trend trading with multiple entries: Enter first position on trend line break. Add on pullback to 20 EMA. Add on break of swing high. Trail stop to each new swing low. Never risk more than 6% total.", 4)

add("dicks-forex-secrets", "MacroAnalysis", ["Trending"],
    "Cross rates: EUR/JPY, GBP/JPY, EUR/GBP provide diversification. Crosses often trend cleaner than majors because less algorithmic noise. But spreads are wider - factor into profitability.", 3)

add("dicks-forex-secrets", "RiskManagement", ["Trending", "Ranging"],
    "Trailing stop methods: 1) Fixed pip distance 2) ATR-based 3) Swing point (move stop to last higher low in uptrend) 4) Parabolic SAR 5) Chandelier exit. Choose based on timeframe and volatility.", 4)

add("dicks-forex-secrets", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Patience is the #1 edge: Wait for A+ setups. Trading B and C setups leads to losses. If no clear setup exists, the best trade is no trade. Capital preservation > profit generation.", 5)

add("dicks-forex-secrets", "TechnicalAnalysis", ["Trending"],
    "Divergence trading: When price makes new high but RSI/MACD fails to confirm, bearish divergence. When price makes new low but oscillator makes higher low, bullish divergence. Strongest at key S/R zones.", 4)

add("dicks-forex-secrets", "MacroAnalysis", ["BreakingNews"],
    "News impact ranking: 1) NFP 2) Central bank decisions 3) CPI 4) GDP 5) Retail sales. Trade with the trend after news. Avoid counter-trend trades on news days - volatility can extend moves beyond expected levels.", 4)

add("dicks-forex-secrets", "Execution", ["SessionOpen"],
    "London fix (4PM GMT): Large institutional orders cluster here. Price often spikes then reverses. Be aware of fix-related flows, especially on month-end and quarter-end dates when rebalancing occurs.", 4)

# =========================================================================
# CROSS-BOOK SYNTHESIS: Universal FX Edges
# =========================================================================

add("synthesis-universal", "RiskManagement", ["Trending", "Ranging"],
    "Golden rules from all 18 books: 1) Never risk >2% per trade 2) Always use stops 3) Maintain 2:1+ R:R 4) Trade with the trend 5) Keep a journal 6) Backtest before live 7) Start small 8) Be patient.", 5)

add("synthesis-universal", "Execution", ["SessionOpen"],
    "Session strategy matrix: Asian = range identification + carry collection. London = breakout initiation. NY overlap = trend continuation. Late NY = position squaring. Match strategy to session.", 5)

add("synthesis-universal", "Sentiment", ["ExtremeFear", "ExtremeGreed"],
    "Contrarian sentiment signals: Magazine covers, extreme COT positioning, retail SSI >80% one-sided, news headlines with extreme language (surge/plummet), Google Trends peaks. All signal trend exhaustion.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Confluence is king: Single signals fail often. Highest probability setups combine: 1) Key S/R zone 2) Trend direction 3) Candlestick pattern 4) Indicator confirmation 5) Session timing. More confluence = higher win rate.", 5)

add("synthesis-universal", "MacroAnalysis", ["Trending"],
    "Interest rate differentials are the primary long-term FX driver. Central bank divergence creates multi-month trends. Monitor: Fed funds futures, ECB deposit rate expectations, BOJ yield curve control.", 5)

add("synthesis-universal", "RegimeDetection", ["Trending", "Ranging"],
    "Regime detection tools: ADX>25 = trending (use trend strategies). ADX<25 = ranging (use range strategies). Bollinger bandwidth for volatility regime. Correlation stability for risk regime.", 5)

add("synthesis-universal", "RiskManagement", ["HighVolatility", "LowVolatility"],
    "Volatility-based position sizing: Use ATR(14) to normalize position sizes across pairs. Higher ATR = smaller position, lower ATR = larger position. This equalizes risk contribution from each trade.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Best FX chart patterns by reliability: 1) Flags/pennants (continuation) 2) Triangles (breakout) 3) Head & shoulders (reversal) 4) Double top/bottom (reversal) 5) Channels (range). Daily+ timeframes most reliable.", 4)

add("synthesis-universal", "Execution", ["SessionOpen", "SessionClose"],
    "Month-end/quarter-end flows: Institutional rebalancing creates predictable flows. USD tends to weaken month-end due to portfolio rebalancing. Quarter-end effects are stronger. London fix is key timing.", 4)

add("synthesis-universal", "Sentiment", ["ExtremeFear", "ExtremeGreed"],
    "Risk-on/Risk-off (RORO) framework: In risk-on, buy AUD/JPY, NZD/JPY, sell USD/CHF. In risk-off, buy USD/JPY (sometimes), CHF/JPY, sell AUD/USD. VIX is the RORO thermometer.", 5)

add("synthesis-universal", "MacroAnalysis", ["FomcDate"],
    "FOMC trading framework: Pre-FOMC, reduce exposure. Post-FOMC, trade the statement language shift (hawkish vs dovish). Watch for 'key phrases' that signal policy direction change. Dollar tends to rally on hawkish surprises.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Moving average systems that work: 10/20 EMA crossover with 200 SMA filter for trend. Enter on pullback to 20 EMA after crossover. Stop below recent swing. Trail using swing points. Works on 4H and daily.", 4)

add("synthesis-universal", "RiskManagement", ["Trending", "Ranging"],
    "Maximum drawdown rules: If drawdown reaches 10%, reduce position size by 50%. If 15%, stop trading for 1 week. If 20%, stop for 1 month and review entire system. Capital preservation is paramount.", 5)

add("synthesis-universal", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "The 3 enemies: 1) Overtrading (most common) 2) Oversizing (most destructive) 3) Overconfidence (most dangerous after winning streak). Combat with rules, journaling, and mandatory breaks.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Price action > indicators: All indicators are derived from price with lag. Support/resistance zones, candlestick patterns, and market structure provide faster, more reliable signals. Use indicators only for confirmation.", 4)

add("synthesis-universal", "MacroAnalysis", ["Trending"],
    "Commodity currency playbook: AUD tracks gold/copper. CAD tracks crude oil. NZD tracks dairy. When commodities trend, commodity currencies follow with 1-2 day lag. Use commodity charts as leading indicator.", 4)

add("synthesis-universal", "Execution", ["BreakingNews"],
    "NFP trading playbook: 1) Close positions 30 min before 2) Wait 15 min after for dust to settle 3) Trade the secondary move (not the spike) 4) Use wider stops (1.5x normal) 5) Reduce position size by 50%.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Ranging"],
    "Range trading checklist: 1) ADX < 25 2) Clear horizontal S/R 3) No major news pending 4) Oscillating RSI between 30-70 5) Price in middle third of range (not at edges) for entry. Edge is in buying support, selling resistance.", 5)

add("synthesis-universal", "RiskManagement", ["Trending", "Ranging"],
    "Risk of ruin calculation: With 2% risk per trade, 50% win rate, and 2:1 R:R, risk of ruin is <1%. With 5% risk per trade, same stats, risk of ruin jumps to 15%. Position sizing is the difference between survival and ruin.", 5)

# =========================================================================
# ADDITIONAL EXTRACTED KNOWLEDGE - Deep dives from each book
# =========================================================================

# Book 1: More Kathy Lien strategies
add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending"],
    "Moving average envelope: Add 2% bands above/below 20 SMA. In trending markets, price rides the upper band (uptrend) or lower band (downtrend). Break below upper band in uptrend = potential trend end.", 3)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Ranging"],
    "RSI range trading: When RSI oscillates between 30-70 for 20+ bars, market is range-bound. Buy RSI 30 with stop below range low. Sell RSI 70 with stop above range high. Exit at opposite extreme.", 4)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending"],
    "PPP-based trading: When a currency pair deviates >20% from PPP fair value, look for mean reversion trades. OECD publishes PPP estimates. Combine with technical confirmation for timing.", 3)

add("kathy-lien-currency-market", "Execution", ["BreakingNews"],
    "Straddle strategy for news: Place buy stop 30 pips above current price and sell stop 30 pips below. Use OCO (one-cancels-other) orders. One triggers on news spike. Cancel other. Risk: whipsaw both stops.", 4)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending"],
    "Bollinger Band %B: When %B > 1, price is above upper band (overbought in range, strong in trend). When %B < 0, price is below lower band. Use with ADX to distinguish trend from range.", 3)

add("kathy-lien-currency-market", "TechnicalAnalysis", ["Trending"],
    "MACD histogram divergence: When MACD histogram makes lower highs while price makes higher highs, momentum is fading. Enter short when histogram turns negative. Strongest signal at key resistance.", 4)

add("kathy-lien-currency-market", "MacroAnalysis", ["Trending"],
    "Twin deficit countries (trade deficit + budget deficit) see long-term currency depreciation. US in 2000s is prime example. Monitor deficit sizes relative to GDP for structural currency direction.", 3)

add("kathy-lien-currency-market", "RegimeDetection", ["Trending", "Ranging"],
    "Bollinger bandwidth squeeze: When bandwidth falls to multi-month low, explosive breakout imminent. Direction unknown - use straddle or wait for break. The longer the squeeze, the bigger the eventual move.", 4)

# Book 2: More intermarket
add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Yield curve and USD: When US yield curve steepens (long-short spread widens), USD tends to strengthen. When curve inverts, recession expectations rise, USD weakens as rate cut expectations increase.", 3)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Copper/Gold ratio as growth indicator: Rising ratio = growth optimism (AUD bullish). Falling ratio = growth pessimism (JPY/CHF bullish). Leading indicator for risk sentiment shifts.", 3)

add("kathy-lien-intermarket", "Sentiment", ["ExtremeFear"],
    "VIX term structure: When front-month VIX > back-month (backwardation), market expects crisis NOW. This is when carry trades unwind most violently. Go long JPY/CHF, short commodity currencies.", 4)

add("kathy-lien-intermarket", "MacroAnalysis", ["Trending"],
    "Crude oil and CAD correlation: 90-day rolling correlation between crude and USD/CAD often exceeds -0.80. When correlation breaks down, it signals regime change in Canadian economy. Monitor weekly.", 3)

# Book 3: More Twomey
add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Covered interest parity: Forward points = Spot x (r_d - r_f) / (1 + r_f x t). When CIP breaks down, it signals dollar funding stress. 2016 CIP breakdown was early warning of USD funding crisis.", 3)

add("twomey-inside-currency", "OrderFlow", ["SessionOpen"],
    "Order flow at fixings: London 4PM fix accounts for ~40% of daily volume. Month-end and quarter-end fixes see massive rebalancing flows. These flows are predictable and can be anticipated.", 4)

add("twomey-inside-currency", "TechnicalAnalysis", ["Trending"],
    "Standard deviation bands: Unlike Bollinger (2 SD), use 1 SD for entries in trends (mean reversion within trend) and 3 SD for extreme overbought/oversold. 2 SD is default but not always optimal.", 3)

add("twomey-inside-currency", "MacroAnalysis", ["Trending"],
    "Portfolio balance effect: When central bank buys government bonds (QE), it pushes investors into riskier assets including foreign bonds. This weakens the currency. QE exit strengthens currency.", 4)

# Book 4: More Saettele
add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "RSI failure swings: Bullish failure swing: RSI falls below 30, bounces above 30, pulls back but stays above 30, then breaks above prior high. Bearish failure swing is mirror image. Stronger than simple OB/OS.", 4)

add("saettele-sentiment-forex", "Sentiment", ["ExtremeFear", "ExtremeGreed"],
    "Sentiment extreme checklist: 1) Magazine cover featuring trend 2) COT extreme 3) RSI divergence 4) News headline extreme language 5) Google Trends peak. 3+ signals simultaneously = very high conviction reversal.", 5)

add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "Elliott Wave guideline of alternation: If wave 2 is sharp, wave 4 will be sideways, and vice versa. Helps anticipate the nature of the next correction. Wave 2 typically retraces 50-61.8% of wave 1.", 3)

add("saettele-sentiment-forex", "Sentiment", ["Trending"],
    "Options risk reversals: When 25-delta risk reversal is extremely negative for a currency, market is paying heavily for puts = extreme bearish sentiment. Contrarian signal for reversal.", 4)

add("saettele-sentiment-forex", "TechnicalAnalysis", ["Trending"],
    "Slow stochastics in FX: Default 14,3,3 settings. Buy when %K crosses above %D below 20 AND price is at support zone. Sell when %K crosses below %D above 80 AND price is at resistance zone.", 3)

# Book 5: More Ponsi
add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Trending"],
    "The Rubber Band strategy: When price deviates >2 standard deviations from 20 SMA, expect mean reversion. Enter counter-trend with tight stop beyond the extreme. Target: 20 SMA. Works in ranging markets.", 4)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Ranging"],
    "Inside day + Bollinger squeeze: When inside days form during Bollinger Band contraction, the breakout will be powerful. Trade in direction of the break. Stop at opposite side of inside day range.", 4)

add("ponsi-patterns-probabilities", "Execution", ["SessionOpen"],
    "Frankfurt open (2AM EST) often sets the daily tone for EUR pairs. If EUR/USD makes a new daily high at Frankfurt open, bullish bias for London. If new daily low, bearish bias.", 3)

add("ponsi-patterns-probabilities", "TechnicalAnalysis", ["Trending"],
    "Momentum ignition: Sharp price spike in thin liquidity (Asian session) that quickly reverses. Don't chase these moves. Wait for London confirmation. False breakouts are more common than real ones in Asia.", 4)

add("ponsi-patterns-probabilities", "RiskManagement", ["Trending", "Ranging"],
    "Daily loss limit: If you lose 3% of account in a single day, stop trading for the day. If you lose 5% in a week, stop for the weekend. Prevents emotional spiral of revenge trading.", 5)

# Book 6: More Michalowski
add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "Bar-by-bar analysis: Read each candle in context of prior candles. Series of higher highs/lows with strong closes = trend continuation. Lower highs with long upper wicks = distribution, reversal warning.", 4)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Trending"],
    "100-bar MA slope: Steep slope = strong trend, look for pullback entries. Flat slope = range, use oscillators. MA crossing price frequently = choppy, avoid trading.", 3)

add("michalowski-attacking-trends", "RiskManagement", ["Trending"],
    "Scaling into trends: Enter 1/3 position on breakout, add 1/3 on pullback to 20 EMA, add final 1/3 on continuation above breakout high. Trail stop to breakeven after first add.", 4)

add("michalowski-attacking-trends", "TechnicalAnalysis", ["Ranging"],
    "Failed breakout reversal: When breakout fails to follow through within 3-5 candles, enter counter-trend. Stop beyond the false breakout high/low. Failure to follow through = trapped traders.", 4)

# Book 7: More Naked Forex
add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "Zone strength indicators: 1) More touches = stronger zone 2) Older zones are recycled 3) Zones at round numbers are stronger 4) Zones confirmed on multiple timeframes are strongest.", 4)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "The Last Kiss entry: After zone break, price returns to test zone from opposite side. Enter when price touches zone and shows rejection (pin bar, engulfing). Stop beyond zone. High probability because trapped traders provide fuel.", 5)

add("nekritin-naked-forex", "RiskManagement", ["Trending", "Ranging"],
    "Expert traders risk less, not more. Beginners risk 5-10% per trade. Experts risk 0.5-1%. The edge is in the setup quality, not the position size. Size kills the unskilled.", 5)

add("nekritin-naked-forex", "Psychology", ["Trending", "Ranging"],
    "The 10,000 hour rule: Naked trading requires screen time. Manual backtesting accelerates learning. 200+ manual backtests = beginning of expertise. 1000+ = proficiency. There are no shortcuts.", 4)

add("nekritin-naked-forex", "TechnicalAnalysis", ["Trending"],
    "Zone + candle pattern = naked signal. Never take a candle pattern without zone context. A kangaroo tail in the middle of nowhere has low probability. At a key zone, it's powerful.", 5)

# Book 8: More Little Book
add("kathy-lien-little-book", "MacroAnalysis", ["Trending"],
    "US dollar index (DXY) as compass: DXY trend sets the tone for all USD pairs. If DXY is rising, focus on shorting EUR/USD, GBP/USD, AUD/USD. If falling, focus on buying them.", 4)

add("kathy-lien-little-book", "TechnicalAnalysis", ["Trending"],
    "Trend trading edge: In FX, trends persist because of interest rate carry and institutional momentum. A simple 200 SMA crossover system has positive expectancy over 10+ year backtests.", 4)

add("kathy-lien-little-book", "Execution", ["BreakingNews"],
    "Economic data trading: For proactive trading, enter 15-20 min before data based on forecast. For reactive trading, wait 15 min after release, trade the direction that holds. Never trade during the initial spike.", 4)

add("kathy-lien-little-book", "MacroAnalysis", ["Trending"],
    "Risk-on currencies: AUD, NZD, MXN. Risk-off currencies: JPY, CHF, USD. Monitor equity markets, credit spreads, and VIX for risk regime. Trade accordingly.", 4)

# Book 9: More Sensible Guide
add("wachtel-sensible-forex", "RiskManagement", ["Trending", "Ranging"],
    "Tiered leverage approach: Beginners max 5:1. Intermediate 10:1. Advanced 20:1. Professional 50:1+. Scale leverage with experience and track record. Never jump to high leverage without proven system.", 4)

add("wachtel-sensible-forex", "TechnicalAnalysis", ["Trending"],
    "Trend identification method: Weekly chart determines primary trend. Daily determines intermediate trend. 4H determines short-term trend. All three must agree for highest conviction trades.", 4)

add("wachtel-sensible-forex", "MacroAnalysis", ["Trending"],
    "Political risk premium: Elections, referendums, and political crises add volatility premium to currencies. Reduce position size 50% ahead of known political events. GBP during Brexit is the textbook example.", 3)

add("wachtel-sensible-forex", "RiskManagement", ["HighVolatility"],
    "Guaranteed stops: Some brokers offer guaranteed stops for a premium. Use them ahead of binary events (NFP, elections). Prevents gap risk and slippage on catastrophic moves.", 3)

# Book 10: More Chen
add("chen-essentials-fx", "TechnicalAnalysis", ["Trending"],
    "Wedge patterns: Rising wedge in uptrend = bearish reversal. Falling wedge in downtrend = bullish reversal. Enter on break of wedge boundary. Target = base of wedge projected from break point.", 3)

add("chen-essentials-fx", "TechnicalAnalysis", ["Ranging"],
    "Rectangle/box pattern: Price bouncing between parallel horizontal lines. Buy at support, sell at resistance. Breakout direction determines trend. Target = height of rectangle projected from break.", 3)

add("chen-essentials-fx", "MacroAnalysis", ["Trending"],
    "Carry trade unwinding: When VIX spikes above 25, carry trades unwind rapidly. High-yielders fall, low-yielders rise. This creates 500-1000 pip moves in days. Risk management must account for this tail risk.", 4)

add("chen-essentials-fx", "Execution", ["SessionOpen"],
    "Asian breakout strategy: Identify Asian session range for AUD/USD or NZD/USD (typically 20-40 pips). Place orders 10 pips beyond range. London open often triggers the breakout.", 4)

# Book 11: More Smith
add("smith-living-trading-fx", "TechnicalAnalysis", ["Trending"],
    "ADX trend strength: ADX 20-25 = developing trend. ADX 25-40 = strong trend. ADX 40-50 = very strong. ADX > 50 = extreme (expect reversal). Use ADX direction (rising/falling) more than absolute level.", 4)

add("smith-living-trading-fx", "TechnicalAnalysis", ["Ranging"],
    "Keltner channels: Use ATR-based channels instead of Bollinger. Price breaking above upper channel in uptrend = continuation. Price returning to middle channel = potential entry. Less noise than Bollinger.", 3)

add("smith-living-trading-fx", "Psychology", ["Trending", "Ranging"],
    "The 3-trade rule: After 3 consecutive losses, stop trading for 24 hours. Review the losses. Were they following your rules? If yes, variance. If no, fix the behavior before resuming.", 4)

add("smith-living-trading-fx", "MacroAnalysis", ["Trending"],
    "Seasonal patterns: 'Sell in May and go away' partially applies to FX. Summer months (June-August) see reduced volatility. September-November often sees strongest trending moves.", 3)

# Book 12: More Henderson
add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "FX reserves as policy tool: Central bank FX intervention effectiveness depends on: 1) Sterilized vs unsterilized 2) Market conditions 3) Policy credibility. Unsterilized intervention has lasting impact.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Global imbalances: Current account surpluses in Asia/Germany and deficits in US/UK create structural currency flows. These imbalances resolve through currency adjustment over years, not months.", 3)

add("henderson-currency-strategy", "MacroAnalysis", ["Trending"],
    "Productivity convergence: Developing nations with faster productivity growth see real exchange rate appreciation. This explains long-term appreciation trend of Asian currencies against USD.", 3)

add("henderson-currency-strategy", "RiskManagement", ["HighVolatility"],
    "Tail risk hedging: Buy far OTM options on carry positions. Cost is minimal (0.5-1% annually) but provides catastrophic protection. Like insurance - seems wasteful until you need it.", 4)

# Book 13: More Singh
add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "ADX + DI system: +DI crossing above -DI = buy signal. -DI crossing above +DI = sell signal. Only trade when ADX > 20 (trending). When ADX falls below 20, trend is exhausted.", 4)

add("singh-17-strategies", "TechnicalAnalysis", ["Trending"],
    "Donchian channel breakout: Buy when price breaks above 20-period high. Sell when breaks below 20-period low. Trail stop at 10-period low (longs) or 10-period high (shorts). Classic turtle system adapted for FX.", 4)

add("singh-17-strategies", "TechnicalAnalysis", ["Ranging"],
    "RSI + Bollinger combo: Buy when RSI < 30 AND price touches lower Bollinger Band. Sell when RSI > 70 AND price touches upper band. Exit at middle band (20 SMA). Only in range-bound conditions.", 3)

add("singh-17-strategies", "RiskManagement", ["Trending", "Ranging"],
    "Anti-martingale: Double position size after winners, halve after losers. This compounds winning streaks and limits losing streak damage. Opposite of martingale which destroys accounts.", 4)

# Book 14: More basics
add("forex-basics-secrets", "TechnicalAnalysis", ["Trending"],
    "Trend line trading: Draw trend line connecting 2-3 swing lows in uptrend. Enter on 3rd touch of trend line with bullish candle. Stop below trend line. Break of trend line signals trend end.", 4)

add("forex-basics-secrets", "TechnicalAnalysis", ["Trending"],
    "Moving average fan: Draw 10, 20, 50 SMA. When they fan out in order, trend is strong. When they converge and cross, trend is changing. The 'fan' visual is quick way to assess trend health.", 3)

add("forex-basics-secrets", "MacroAnalysis", ["BreakingNews"],
    "Central bank surprise: Unexpected rate cuts/hikes cause 200-500 pip moves. These are the most profitable news events to trade. But they're rare. The surprise element is what creates the move.", 4)

# Book 15: More Coulling
add("coulling-forex-beginners", "TechnicalAnalysis", ["Trending"],
    "Wyckoff accumulation schematic: Price tests support multiple times with decreasing volume. Spring (false break below support) marks the end of accumulation. Enter long on spring reversal with stop below spring low.", 4)

add("coulling-forex-beginners", "TechnicalAnalysis", ["Trending"],
    "Supply and demand zones: Sharp moves away from a price level indicate strong supply or demand. The zone where the move started is where institutional orders sit. Return to zone = high probability entry.", 4)

add("coulling-forex-beginners", "RiskManagement", ["Trending", "Ranging"],
    "Risk per trade for different account sizes: $1000 account = 3% ok (small account needs growth). $10,000 = 2%. $100,000+ = 1%. Larger accounts should take proportionally less risk.", 3)

# Book 16: More Dummies
add("brooks-dolan-dummies", "TechnicalAnalysis", ["Trending"],
    "Flag pattern trading: After strong move (flagpole), price consolidates in slight counter-trend channel (flag). Enter on break of flag in direction of flagpole. Target = flagpole length projected from flag break.", 4)

add("brooks-dolan-dummies", "TechnicalAnalysis", ["Ranging"],
    "Pivot point bounce: In ranging markets, price often bounces off pivot levels. Buy at S1 with stop at S2, target pivot or R1. Sell at R1 with stop at R2, target pivot or S1.", 3)

add("brooks-dolan-dummies", "MacroAnalysis", ["Trending"],
    "Commodity price transmission: Rising oil prices → CAD strengthens (oil exporter). Rising gold → AUD strengthens (gold exporter). Rising food prices → NZD strengthens (dairy exporter). 1-3 day lag.", 3)

# Book 17: More Martinez
add("martinez-10-essentials", "TechnicalAnalysis", ["Trending"],
    "The 'W' bottom pattern: Double bottom where second low is slightly higher than first. Volume decreases on second test. Break above neckline (middle high) confirms. Target = bottom to neckline distance.", 3)

add("martinez-10-essentials", "RiskManagement", ["Trending", "Ranging"],
    "Scaling out: At 1:1 R:R, close 50% and move stop to breakeven. At 2:1, close 25% and trail remaining 25%. This guarantees profit while allowing the trade to run.", 4)

# Book 18: More Dicks
add("dicks-forex-secrets", "TechnicalAnalysis", ["Trending"],
    "Price rejection candles: Long wick (>2x body) at key level shows strong rejection. Enter in direction of rejection. Stop at wick tip. Works on all timeframes but most reliable on 4H and daily.", 4)

add("dicks-forex-secrets", "MacroAnalysis", ["FomcDate"],
    "Fed dot plot trading: Individual FOMC members' rate projections. Median dot moving higher = hawkish shift (USD bullish). Median dot moving lower = dovish shift (USD bearish). More impactful than actual rate decision.", 4)

add("dicks-forex-secrets", "TechnicalAnalysis", ["Ranging"],
    "Inside bar breakout: Small candle within prior candle range. High compression = high energy. Enter on break of mother candle (the larger candle). Stop at opposite end of mother candle. Works on all TFs.", 3)

add("dicks-forex-secrets", "Execution", ["SessionOpen", "SessionClose"],
    "Weekend gap trading: Gaps on Sunday open tend to fill within first few hours. If AUD/USD gaps down 30 pips at Sunday open, probability favors gap fill by Monday London. Trade gap fills with tight stops.", 3)

add("dicks-forex-secrets", "MacroAnalysis", ["Trending"],
    "EM carry trade basket: Equal weight long BRL, ZAR, TRY vs short JPY. High yield but extreme tail risk. Only viable with strict risk management and small allocation (max 5% of portfolio).", 3)

# Additional synthesis units
add("synthesis-universal", "TechnicalAnalysis", ["Trending", "Ranging"],
    "Entry checklist (all books agree): 1) Define trend (200 SMA) 2) Identify key zone (S/R) 3) Wait for price to reach zone 4) Look for rejection candle pattern 5) Confirm with indicator 6) Calculate position size 7) Set stop and target.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Best FX trending strategies: 1) Moving average crossover with 200 SMA filter 2) Fibonacci retracement entry at 38.2-61.8% 3) Inside day breakout 4) Perfect Order 5) Channel breakout. All have positive expectancy.", 4)

add("synthesis-universal", "TechnicalAnalysis", ["Ranging"],
    "Best FX range strategies: 1) RSI 30/70 oscillator 2) Bollinger Band reversal 3) Double zero fade 4) Pivot point bounce 5) Stochastic cross at extremes. All require ADX < 25 confirmation.", 4)

add("synthesis-universal", "RiskManagement", ["Trending", "Ranging"],
    "Professional risk framework: 1) Max 2% per trade 2) Max 6% total portfolio heat 3) Max 10% monthly drawdown 4) Daily loss limit 3% 5) Correlation-adjusted sizing 6) ATR-based stops 7) Scale in/out.", 5)

add("synthesis-universal", "MacroAnalysis", ["Trending"],
    "Currency strength ranking: Calculate 20-day return for each major currency. Go long strongest vs shortest. Rebalance weekly. Simple momentum strategy that captures macro trends. Used by macro hedge funds.", 4)

add("synthesis-universal", "Execution", ["SessionOpen"],
    "Asian range breakout system: Monitor 7PM-2AM EST range for EUR/USD. Buy 10 pips above range high at London open. Sell 10 pips below range low. Stop at opposite side of range. Target 1.5x range width.", 4)

add("synthesis-universal", "Psychology", ["Trending", "Ranging"],
    "Trading edge = (Win% x Avg Win) - (Loss% x Avg Loss). To increase edge: 1) Increase R:R (better setups) 2) Increase win rate (better filters) 3) Decrease costs (tighter spreads, less slippage). Focus on all three.", 5)

add("synthesis-universal", "MacroAnalysis", ["FomcDate"],
    "Central bank meeting playbook: 1) Reduce exposure 24h before 2) Don't trade during announcement 3) Wait for statement analysis 4) Trade the language shift, not the rate decision 5) Follow-through usually occurs next day.", 5)

add("synthesis-universal", "Sentiment", ["ExtremeFear", "ExtremeGreed"],
    "Composite sentiment score: COT extreme (weight 30%) + SSI extreme (25%) + options risk reversal (20%) + news headline extreme (15%) + Google Trends peak (10%). Score >80 = high conviction contrarian signal.", 4)

add("synthesis-universal", "Execution", ["SessionOpen", "SessionClose"],
    "Daily routine: 1) Check economic calendar 2) Mark key levels on daily charts 3) Assess trend direction 4) Identify setups during London/NY overlap 5) Execute with proper sizing 6) Journal all trades 7) Review weekly.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Multi-pair confirmation: When EUR/USD and GBP/USD both break resistance, USD weakness is confirmed. When only one breaks, it may be pair-specific. Cross-market confirmation reduces false signals.", 4)

add("synthesis-universal", "RiskManagement", ["HighVolatility"],
    "Volatility regime adjustment: In high VIX (>25) environment, reduce position sizes by 50%, widen stops by 50%, and reduce number of open positions. Capital preservation in storms, aggressive in calm.", 5)

add("synthesis-universal", "MacroAnalysis", ["Trending"],
    "Interest rate cycle positioning: Early cycle (rates bottoming) = buy USD. Mid cycle (rates rising) = buy high-yielders. Late cycle (rates peaking) = buy JPY/CHF. Recession (rates cutting) = buy bonds, sell equities.", 4)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Timeframe hierarchy for FX: Monthly = secular trend. Weekly = primary trend. Daily = intermediate trend. 4H = short-term trend. 1H = entry timing. 15M = fine-tuning. Higher TF always takes precedence.", 5)

add("synthesis-universal", "RegimeDetection", ["Trending", "Ranging"],
    "Market regime identification matrix: Trending+LowVol = trend continuation (best conditions). Trending+HighVol = volatile trend (reduce size). Ranging+LowVol = compression (prepare for breakout). Ranging+HighVol = choppy (avoid trading).", 5)

add("synthesis-universal", "Execution", ["BreakingNews"],
    "Post-news trading: After major data release, wait for first 15-min candle to close. Trade the direction of the close. Use the pre-data range for stop placement. The secondary move is more reliable than the initial spike.", 5)

add("synthesis-universal", "TechnicalAnalysis", ["Trending"],
    "Moving average as trailing stop: In strong trends, use 20 EMA as trailing stop on daily chart. When daily close below 20 EMA, exit. Simple, objective, and captures majority of trend moves.", 4)

add("synthesis-universal", "RiskManagement", ["Trending", "Ranging"],
    "Compounding effect: Starting with $10,000, risking 2% per trade with 2:1 R:R and 50% win rate, account grows to ~$14,000 in year 1. Same system with 5% risk leads to ruin in year 2. Math matters.", 5)

add("synthesis-universal", "MacroAnalysis", ["Trending"],
    "FX is a relative game: Currency strength is always relative to another. Don't analyze one currency in isolation. Analyze the PAIR. Strong economy + weak economy = strong trend. Two strong economies = ranging.", 5)

add("synthesis-universal", "Psychology", ["ExtremeFear", "ExtremeGreed"],
    "Tilt prevention: After 3 losses, pause. After big win, pause. After breaking a rule, pause. The pause prevents emotional cascade. Set hard rules for mandatory breaks. Follow them mechanically.", 5)

# Write all knowledge units to JSON
output_path = r"C:\Users\spenc\dev\savant-trading\knowledge\book_forex_complete.json"
with open(output_path, 'w', encoding='utf-8') as f:
    json.dump(knowledge_units, f, indent=2, ensure_ascii=False)

print(f"Generated {len(knowledge_units)} knowledge units")
print(f"Written to {output_path}")

# Summary stats
sources = {}
topics = {}
conditions = {}
for ku in knowledge_units:
    sources[ku['source']] = sources.get(ku['source'], 0) + 1
    topics[ku['topic']] = topics.get(ku['topic'], 0) + 1
    for c in ku['conditions']:
        conditions[c] = conditions.get(c, 0) + 1

print(f"\nSources: {len(sources)}")
for s, c in sorted(sources.items(), key=lambda x: -x[1]):
    print(f"  {s}: {c}")

print(f"\nTopics: {len(topics)}")
for t, c in sorted(topics.items(), key=lambda x: -x[1]):
    print(f"  {t}: {c}")

print(f"\nConditions: {len(conditions)}")
for c, n in sorted(conditions.items(), key=lambda x: -x[1]):
    print(f"  {c}: {n}")

priorities = {}
for ku in knowledge_units:
    p = ku['priority']
    priorities[p] = priorities.get(p, 0) + 1
print(f"\nPriority distribution:")
for p, c in sorted(priorities.items()):
    print(f"  P{p}: {c}")
