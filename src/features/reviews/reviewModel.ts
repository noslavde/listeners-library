/**
 * Local review contract, validation and rating order. Store integer tenths
 * (0..50), display 0.0..5.0, and treat null as unrated rather than zero.
 */

export type Review = { rating: number | null; text: string };
/**
 * Validates integer-tenths range and UTF-8 text length. Allows rating-only or
 * review-only entries but rejects an entirely empty record.
 */
export function reviewError(value: Review): string {
  if (
    value.rating !== null &&
    (!Number.isInteger(value.rating) || value.rating < 0 || value.rating > 50)
  )
    return "Choose a rating from 0.0 to 5.0 in steps of 0.1.";
  if (new TextEncoder().encode(value.text).length > 10000)
    return "Keep your review within 10,000 bytes.";
  if (value.rating === null && !value.text.trim())
    return "Add a rating or write a review.";
  return "";
}

/**
 * Highest rating first, with unrated below a real zero. Equal ratings preserve
 * the caller's existing stable ordering.
 */
export function ratingOrder(a: Review | undefined, b: Review | undefined) {
  return (b?.rating ?? -1) - (a?.rating ?? -1);
}
