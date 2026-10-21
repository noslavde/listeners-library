/**
 * Library-series navigation with an accessible selected state. The series list
 * describes owned/cached books, not a separate Audible store catalog.
 */

/**
 * Exposes All titles and owned-series filters; selection is reflected with
 * accessible pressed state and themed styling.
 */
export function ExploreNav({
  series,
  selected,
  onSelect,
}: {
  series: string[];
  selected: string;
  onSelect: (series: string) => void;
}) {
  return (
    <aside className="explore-nav" aria-label="Explore library">
      <h2>Explore</h2>
      {["", ...series].map((name) => (
        <button
          key={name}
          aria-pressed={selected === name}
          onClick={() => onSelect(name)}
        >
          {selected === name && <span aria-hidden="true">● </span>}
          {name || "All titles"}
        </button>
      ))}
    </aside>
  );
}
