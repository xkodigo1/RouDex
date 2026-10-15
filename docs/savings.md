# Savings estimate

The **Estimated savings** card in Summary shows what the last 30 days of
Codex turns cost and what they would have cost if every turn had run on the
`fuerte` tier. RouDex has no prices of its own: you add them to `router.toml`
(in `~/.roudex`).

## Prices

Prices are in USD per million tokens, per model:

```toml
[prices."gpt-6.1-sol"]
input = 2.0
cached_input = 0.5
output = 10.0

[prices."gpt-6-luna"]
input = 0.5
cached_input = 0.1
output = 2.0
```

Use whatever reference you trust, such as the API price list. On a ChatGPT
plan Codex spends quota rather than money, so read the result as a relative
measure.

Without prices the card shows the tokens of each tier. It also shows the
lines to add for each model that still has no price.

## How it is computed

- **Cost of a turn:**
  `(input − cached) × input + cached × cached_input + output × output`,
  divided by one million. As in Codex's counters, input includes the cached
  part and output includes reasoning.
- **At fuerte:** the same tokens at the price of the model that the `fuerte`
  tier has today in `router.toml`.
- **Router cost:** what the classifier spent to route the prompts it decided.
  Since 0.9.0 it records its own tokens.
- **Savings** = cost at fuerte − cost − router cost.
- **Unpriced turns:** a turn whose model, or the `fuerte` model, has no price
  is counted apart and left out. The card names those models.

It is an estimate. On another model the same work would have used other
tokens.

## Sources

- **chat:** the turns of RouDex threads.
- **tasks:** the turns of spec task runs. Their token totals come from the
  evidence each run keeps.
- **Codex Desktop (optional):** turn on **include Codex Desktop** in the card,
  or set `[savings] include_codex_desktop = true`.
  - RouDex then reads the session files in `~/.codex/sessions` (or
    `$CODEX_HOME/sessions`) from the last 30 days.
  - From each file it parses only the lines with the model of each turn and
    the token counts of each request. It never parses or keeps the
    conversation.
  - Files that did not change since the last read are skipped.
  - Threads RouDex ran itself are not counted twice.
  - With the option off, no session file is opened.
