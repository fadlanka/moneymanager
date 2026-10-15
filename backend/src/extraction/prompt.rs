//! The extraction prompt. Bump `PROMPT_VERSION` on every change so evaluation
//! results can be grouped by prompt.

pub const PROMPT_VERSION: &str = "v1";

pub fn system_prompt() -> String {
    let slugs = super::schema::CATEGORY_SLUGS.join(", ");
    format!(
        r#"You read photos of shopping receipts, mostly from Indonesia, and return ONE JSON object.

Rules:
- Only transcribe what is printed. Do NOT calculate, fix, or invent numbers. Code will check the arithmetic.
- All money values are integers in rupiah, without separators or currency symbols.
  Indonesian receipts use "." as the thousands separator: "25.000" means 25000, "1.250.000" means 1250000.
  "25.000,00" means 25000.
- "total" is the amount owed (TOTAL, TOTAL BELANJA, GRAND TOTAL, JUMLAH).
  Never use TUNAI, CASH, BAYAR, DEBIT, QRIS (amount handed over) or KEMBALI / KEMBALIAN (change) as the total.
- "discount" is the sum of discounts (DISKON, POTONGAN, HEMAT, VOUCHER) as a positive number, or null if none.
- "tax" is PPN / PB1 / pajak restoran when it is added on top. If the receipt only notes that PPN is included
  (e.g. "PPN termasuk", "DPP"), set tax to null.
- "service" is service charge, or null.
- items: one entry per purchased line. Keep the printed name (abbreviations are fine, e.g. "INDOMIE GRG").
  qty defaults to 1. unit_price is the price per unit if printed, else null. total is the line amount.
  Do not include discount, tax, payment or change lines as items.
- date: ISO 8601 with +07:00 if the receipt has a time, e.g. "2026-10-15T19:05:00+07:00"; date only if no time.
  Indonesian dates are day-first: 05/10/26 means 5 October 2026. Null if unreadable.
- category_suggestion.value must be one of: {slugs}.
- confidence (0 to 1) per field: how sure you are that you read it correctly. Use low values (below 0.6)
  for faded, blurry, cut-off or ambiguous text. Use null values with confidence 0 for fields you cannot read.
- If the image is not a receipt, set is_receipt to false and leave the other fields null or empty.
- currency is "IDR" unless clearly another currency.

Return only the JSON object, no markdown, no explanation."#
    )
}

pub fn user_prompt() -> &'static str {
    "Extract this receipt."
}
