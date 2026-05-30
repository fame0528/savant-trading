## Universal Starter Prompt

```
You are now operating under the ECHO Protocol v4.0.0.

MANDATORY BOOT SEQUENCE — DO NOT BEGIN ANY WORK UNTIL COMPLETE:

1. Read ECHO.md in full. Confirm you have read it by listing ALL 15 Laws
   by NUMBER and NAME. Do not paraphrase — use the exact names.

2. Read protocol.config.yaml and confirm:
   - The configured language
   - All 6 validation commands (build, test, type_check, lint, format, clean)
   - The max_file_lines setting

3. Read coding-standards/{language}.md where {language} is the value from
   protocol.config.yaml. Confirm the naming convention for:
   - Structs/Classes
   - Functions
   - Constants
   - Files

4. State the 5 states of the Perfection Loop FSM in order.

5. State the circuit breaker rules (all 5).

6. List all path entries from protocol.config.yaml.

DO NOT begin any coding, analysis, or task work until you have completed
steps 1-6 and I have confirmed your boot sequence is correct.

After confirmation, maintain strict compliance with all laws throughout
our session. Create FIDs for any violations. Run the Perfection Loop on
every code change. Generate a session summary at the end.
```

---

## Language-Specific Variants

### For Rust Projects

```
You are now operating under the ECHO Protocol v4.0.0 for a RUST project.

BOOT SEQUENCE:
1. Read ECHO.md — list all 15 laws by number and exact name
2. Read protocol.config.yaml — confirm language is "rust", list all commands
3. Read coding-standards/rust.md — confirm: structs, functions, constants, files naming
4. State the Perfection Loop FSM (5 states in order)
5. State all 5 circuit breaker rules
6. Confirm max_file_lines, max_function_lines, and max_line_length from config

CRITICAL RUST RULES:
- No unwrap() in non-test code
- All errors use Result<T, E> with typed errors
- Prefer borrowing over cloning
- One import group: std → external crates → local

DO NOT begin work until boot sequence is verified.
```

### For TypeScript Projects

```
You are now operating under the ECHO Protocol v4.0.0 for a TYPESCRIPT project.

BOOT SEQUENCE:
1. Read ECHO.md — list all 15 laws by number and exact name
2. Read protocol.config.yaml — confirm language is "typescript", list all commands
3. Read coding-standards/typescript.md — confirm: interfaces, functions, constants, files naming
4. State the Perfection Loop FSM (5 states in order)
5. State all 5 circuit breaker rules
6. Confirm max_file_lines and max_line_length from config

CRITICAL TYPESCRIPT RULES:
- strict: true required in tsconfig
- No `any` type — use `unknown` and narrow
- Named exports only, no defaults
- Prefer interface over type for object shapes

DO NOT begin work until boot sequence is verified.
```

### For Python Projects

```
You are now operating under the ECHO Protocol v4.0.0 for a PYTHON project.

BOOT SEQUENCE:
1. Read ECHO.md — list all 15 laws by number and exact name
2. Read protocol.config.yaml — confirm language is "python", list all commands
3. Read coding-standards/python.md — confirm: classes, functions, constants, files naming
4. State the Perfection Loop FSM (5 states in order)
5. State all 5 circuit breaker rules
6. Confirm max_file_lines and max_line_length from config

CRITICAL PYTHON RULES:
- Type hints on all public functions
- No bare except: — always specify exception type
- Use logging module, never print()
- Prefer absolute imports

DO NOT begin work until boot sequence is verified.
```
