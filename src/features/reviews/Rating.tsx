/**
 * Accessible star display for integer-tenths ratings. Null/undefined means
 * unrated; zero is a valid rating and must keep its numeric label.
 */

/**
 * Formats integer tenths for sighted and screen-reader users; null stays
 * Unrated, while zero renders 0.0 / 5.
 */
export function Rating({ rating }: { rating: number | null | undefined }) {
  return (
    <span
      className="local-rating"
      aria-label={
        rating == null
          ? "Not rated locally"
          : `Your rating: ${(rating / 10).toFixed(1)} out of 5`
      }
    >
      <span
        aria-hidden="true"
        className="rating-stars"
        style={{
          backgroundImage: `linear-gradient(90deg,var(--accent) ${(rating ?? 0) * 2}%,var(--muted) ${(rating ?? 0) * 2}%)`,
        }}
      >
        ★★★★★
      </span>
      <span>
        {rating == null ? "Unrated" : `${(rating / 10).toFixed(1)} / 5`}
      </span>
    </span>
  );
}
