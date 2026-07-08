/**
 * v0.25.3: Shared money formatter for the frontend.
 *
 * Previously, 4 separate pages each defined their own fmtMoney lambda:
 *   - Dashboard.tsx:     `Rs. ${n.toFixed(0)}`
 *   - Agents.tsx:        `Rs. ${n.toFixed(0)}`
 *   - ShareCenter.tsx:   `Rs. ${n.toFixed(0)}`
 *   - PurchaseTrips.tsx: `Rs. ${n.toFixed(2)}`  ← different precision!
 *
 * The PurchaseTrips version showed paisa (Rs. 2500.00) while all other
 * pages showed whole rupees (Rs. 2500). This is now consolidated to a
 * single function with consistent toFixed(0) formatting.
 *
 * The backend has utils::format_money which reads currency from
 * business_profile settings. This frontend helper is simpler — it
 * hardcodes "Rs." because the business is PKR-only. If multi-currency
 * support is ever added, this function should call a Rust command
 * instead.
 */

/**
 * Format a number as a Pakistani Rupee string.
 * @example fmtMoney(2500) → "Rs. 2500"
 * @example fmtMoney(2500.99) → "Rs. 2501" (rounded to whole rupees)
 */
export function fmtMoney(n: number): string {
  return `Rs. ${Math.round(n).toLocaleString('en-PK')}`
}
