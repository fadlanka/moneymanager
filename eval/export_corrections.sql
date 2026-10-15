-- One row per confirmed receipt: what the AI read vs what the user saved.
SELECT
    r.id,
    r.provider,
    r.model,
    r.prompt_version,
    r.latency_ms,
    r.input_tokens,
    r.output_tokens,
    jsonb_array_length(r.issues) AS issues,
    (r.extraction -> 'total' ->> 'value')::bigint AS ai_total,
    (r.confirmed ->> 'amount')::bigint AS user_total,
    r.extraction -> 'merchant' ->> 'value' AS ai_merchant,
    r.confirmed ->> 'merchant' AS user_merchant,
    array_to_string(r.corrected_fields, '|') AS corrected_fields
FROM receipts r
WHERE r.status = 'confirmed'
ORDER BY r.created_at;
