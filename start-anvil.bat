@echo off
:: ============================================================
:: Thin root-level wrapper - forwards to start-anvil.bat.
:: start.bat calls "%~dp0start-anvil.bat" expecting a file at root.
:: The canonical launcher lives under scripts\ and owns the full
:: idempotent boot sequence of the Anvil fork plus the prefund
:: transaction. This wrapper preserves the start.bat call site
:: without edits. Idempotent on its own - returns 0 quickly if
:: an existing Anvil is already healthy on :8545.
:: ============================================================
:: NOTE: do NOT introduce `if (...)` blocks inside this file.
:: cmd.exe counts paren chars inside :: lines only when those
:: :: lines sit inside an active if paren block at parse time
:: (LESSON-011 shape). Our wrappers are pure top-level call
:: forwards; if anyone ever adds an if guard here, the call
:: below must be placed outside any paren block OR the comment
:: lines must be swapped from :: to rem to dodge cmd blindspots.
:: ============================================================
call "%~dp0scripts\start-anvil.bat" %*
exit /b %ERRORLEVEL%