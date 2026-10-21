/**
 * Private local rating/review editor and reviewed-book list. Drafts are keyed
 * by book and carry the saved value they were based on; navigating sections
 * does not discard them.
 */

import { useState } from "react";
import { desktop } from "../../playback";
import type { Book } from "../../model";
import { reviewError, type Review } from "./reviewModel";
import { Rating } from "./Rating";
import type { useReviews } from "./useReviews";

type Draft = { rating: string; text: string; expected: Review | null };

/**
 * Keeps independent drafts per book and saves against their original values.
 * Clearing a rating edits the draft; removing a saved review requires a
 * separate confirmation.
 */
export function ReviewsPage({
  books,
  selected,
  onSelect,
  reviews,
}: {
  books: Book[];
  selected: string;
  onSelect: (asin: string) => void;
  reviews: ReturnType<typeof useReviews>;
}) {
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const [busy, setBusy] = useState(false),
    [message, setMessage] = useState("");
  const asin = books.some((b) => b.asin === selected)
    ? selected
    : books[0]?.asin;
  const saved = reviews.entries[asin];
  const draft = drafts[asin] ?? {
    rating: saved?.rating == null ? "" : (saved.rating / 10).toFixed(1),
    text: saved?.text ?? "",
    expected: saved ?? null,
  };
  const value: Review = {
    rating: draft.rating === "" ? null : Math.round(Number(draft.rating) * 10),
    text: draft.text,
  };
  const invalidRating =
    draft.rating !== "" &&
    (!Number.isFinite(Number(draft.rating)) ||
      Math.abs(Number(draft.rating) * 10 - value.rating!) > 0.000001);
  const validation = invalidRating
    ? "Use steps of 0.1 for the rating."
    : reviewError(value);
  function patch(change: Partial<Draft>) {
    setMessage("");
    setDrafts((all) => ({ ...all, [asin]: { ...draft, ...change } }));
  }
  function discard() {
    setDrafts((all) => {
      const next = { ...all };
      delete next[asin];
      return next;
    });
  }
  async function commit(remove = false) {
    setBusy(true);
    setMessage("");
    try {
      await reviews.save(asin, remove ? null : value, draft.expected);
      discard();
      setMessage(remove ? "Local review removed." : "Saved on this PC.");
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="reviews-page">
      <h2>My ratings & reviews</h2>
      <p>
        Private to this account on this PC. Ratings and reviews are encrypted
        locally and never sent to Audible.
      </p>
      {!desktop && <p>Browser demo: reviews last only for this session.</p>}
      {reviews.error && <p role="status">{reviews.error}</p>}
      {!books.length ? (
        <p>Sign in and sync your library to review a book.</p>
      ) : (
        <>
          <label>
            Book
            <select
              aria-label="Book to review"
              value={asin}
              disabled={busy}
              onChange={(e) => {
                onSelect(e.target.value);
                setMessage("");
              }}
            >
              {books.map((b) => (
                <option key={b.asin} value={b.asin}>
                  {b.title}
                </option>
              ))}
            </select>
          </label>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (!validation && !busy) void commit();
            }}
          >
            <Rating rating={value.rating} />
            <label>
              Your rating (0.0–5.0)
              <input
                aria-label="Your rating"
                type="number"
                min="0"
                max="5"
                step="0.1"
                placeholder="Unrated"
                value={draft.rating}
                disabled={busy || !reviews.ready}
                onChange={(e) => patch({ rating: e.target.value })}
              />
            </label>
            <button
              type="button"
              disabled={busy || !reviews.ready}
              onClick={() => patch({ rating: "" })}
            >
              Clear rating
            </button>
            <label>
              Your review
              <textarea
                value={draft.text}
                maxLength={10000}
                disabled={busy || !reviews.ready}
                onChange={(e) => patch({ text: e.target.value })}
                placeholder="What stayed with you?"
              />
            </label>
            <div className="review-actions">
              <button
                className="primary-action"
                disabled={!reviews.ready || busy || !!validation}
              >
                {busy ? "Saving…" : "Save review"}
              </button>
              <button
                type="button"
                disabled={busy || !drafts[asin]}
                onClick={discard}
              >
                Discard changes
              </button>
              <button
                type="button"
                disabled={busy || !reviews.ready || !saved}
                onClick={() => {
                  if (
                    window.confirm(
                      "Remove this PC’s rating and review for this book?",
                    )
                  )
                    void commit(true);
                }}
              >
                Remove review
              </button>
            </div>
            {validation && drafts[asin] && <p>{validation}</p>}
            {message && <p role="status">{message}</p>}
          </form>
        </>
      )}
      <h3>Reviewed books · {Object.keys(reviews.entries).length}</h3>
      <div className="review-list">
        {books
          .filter((b) => reviews.entries[b.asin])
          .map((b) => (
            <article key={b.asin}>
              <h4>{b.title}</h4>
              <Rating rating={reviews.entries[b.asin].rating} />
              <p>{reviews.entries[b.asin].text || "Rating only"}</p>
              <button
                onClick={() => {
                  onSelect(b.asin);
                  setMessage("");
                }}
              >
                Edit review
              </button>
            </article>
          ))}
      </div>
    </section>
  );
}
