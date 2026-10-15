# Evaluation (phase 3)

Goal: measure how well each extraction setup reads Indonesian receipts, per field.

## Data you already collect

Every scanned receipt the user saves stores, in `receipts`:

| column | meaning |
|---|---|
| `extraction` | what the model read, after code validation |
| `confirmed` | what the user actually saved |
| `corrected_fields` | which fields the user changed: `merchant`, `total`, `date`, `category`, `items` |
| `provider`, `model`, `prompt_version` | which setup produced it |
| `latency_ms`, `input_tokens`, `output_tokens` | speed and cost |

So real usage already gives an "edit rate" per field and per model.
`export_corrections.sql` pulls it out:

```bash
docker compose exec -T db psql -U money -d moneymanager -A -F, -f - < eval/export_corrections.sql > eval/corrections.csv
```

## Controlled benchmark (to do)

1. Collect 50–100 receipt photos in `eval/data/images/` (git-ignored: personal data).
2. Label ground truth in `eval/data/labels.jsonl`: merchant, date, total, items.
3. Run each config against the same images: Gemini Flash, a local VLM via Ollama,
   Tesseract OCR + text LLM. Space calls out (rate limit) — no bursts.
4. Report accuracy per field, latency and cost per receipt. Leave unknown results as `...`.

| Setup | Total exact | Date | Merchant | Items F1 | p50 latency | Cost / receipt |
|---|---|---|---|---|---|---|
| Gemini Flash | ... | ... | ... | ... | ... | ... |
| Ollama VLM | ... | ... | ... | ... | ... | ... |
| Tesseract + LLM | ... | ... | ... | ... | ... | ... |
