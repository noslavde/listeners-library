import type { Book } from "./model";
import type { CardPositions } from "./useCardPositions";
import { progressSummary } from "./progress";
function time(seconds: number) {
  return `${Math.floor(seconds / 3600)}:${String(Math.floor(seconds / 60) % 60).padStart(2, "0")}:${String(Math.floor(seconds) % 60).padStart(2, "0")}`;
}
export function CardProgress({
  book,
  saved,
  livePosition,
}: {
  book: Book;
  saved?: CardPositions[string];
  livePosition?: number;
}) {
  const local = saved?.local?.position,
    cloud = saved?.cloud?.position;
  const { position, source, conflict } = progressSummary(saved, livePosition);
  const percent =
    position !== undefined && book.duration > 0
      ? Math.max(0, Math.min(100, (position / book.duration) * 100))
      : null;
  return (
    <div className="card-progress">
      <p>
        {conflict
          ? "Saved positions differ"
          : position === undefined
            ? "Last position unavailable"
            : `${source} ${time(position)}`}
      </p>
      {conflict && (
        <>
          <p>
            This PC {time(local!)} · Audible {time(cloud!)}
          </p>
          <small>
            Resume uses Audible’s official player. Check Saved progress to
            choose a position.
          </small>
        </>
      )}
      {!conflict && livePosition !== undefined && cloud !== undefined && (
        <p>Audible {time(cloud)}</p>
      )}
      {percent !== null && (
        <>
          <progress
            aria-label={`${book.title} listening progress`}
            max={100}
            value={percent}
          />
          <small>{Math.round(percent)}% of book</small>
        </>
      )}
    </div>
  );
}
