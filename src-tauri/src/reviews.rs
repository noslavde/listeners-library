//! Private per-account rating/review persistence with no service calls. Ratings
//! use integer tenths, optimistic expected-value checks protect edits, and all
//! commands require the trusted main window and captured account generation.

//! Private ratings and reviews. No service requests or annotation sync.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub rating: Option<u8>, // Integer tenths: 0..=50; None means unrated.
    pub text: String,
}
#[derive(Serialize)]
pub struct Entry {
    asin: String,
    #[serde(flatten)]
    review: Review,
}
fn schema(c: &Connection) -> Result<(), String> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS local_reviews(asin TEXT PRIMARY KEY,rating INTEGER CHECK(rating BETWEEN 0 AND 50),text TEXT NOT NULL)")
        .map_err(|_| "Cannot open local reviews".into())
}
fn get(c: &Connection, asin: &str) -> Result<Option<Review>, String> {
    c.query_row(
        "SELECT rating,text FROM local_reviews WHERE asin=?1",
        [asin],
        |r| {
            Ok(Review {
                rating: r.get(0)?,
                text: r.get(1)?,
            })
        },
    )
    .optional()
    .map_err(|_| "Cannot read local review".into())
}
fn save(
    c: &Connection,
    asin: &str,
    review: Option<Review>,
    expected: Option<Review>,
) -> Result<(), String> {
    if !crate::valid_asin(asin) {
        return Err("Choose a valid book".into());
    }
    if let Some(r) = &review {
        if r.rating.is_some_and(|n| n > 50)
            || r.text.len() > 10000
            || (r.rating.is_none() && r.text.trim().is_empty())
        {
            return Err(
                "Choose a rating from 0.0 to 5.0 or write a review (up to 10,000 bytes)".into(),
            );
        }
    }
    schema(c)?;
    let owned: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM books WHERE asin=?1)",
            [asin],
            |r| r.get(0),
        )
        .map_err(|_| "Cannot check book")?;
    if !owned {
        return Err("Sync this book before reviewing it".into());
    }
    c.execute_batch("SAVEPOINT local_review")
        .map_err(|_| "Cannot save review")?;
    let result = (|| {
        if get(c, asin)? != expected {
            return Err("This review changed. Reload it before saving.".into());
        }
        if let Some(r) = review {
            c.execute("INSERT INTO local_reviews(asin,rating,text) VALUES(?1,?2,?3) ON CONFLICT(asin) DO UPDATE SET rating=excluded.rating,text=excluded.text", params![asin,r.rating,r.text.trim()]).map_err(|_| "Cannot save review")?;
        } else {
            c.execute("DELETE FROM local_reviews WHERE asin=?1", [asin])
                .map_err(|_| "Cannot remove review")?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = c.execute_batch("ROLLBACK TO local_review");
    }
    c.execute_batch("RELEASE local_review")
        .map_err(|_| "Cannot finish review save")?;
    result
}
#[tauri::command]
/// Read private local reviews for the captured account; no Audible request or
/// annotation sync is involved.
pub fn list_reviews(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
) -> Result<Vec<Entry>, String> {
    crate::local(&window)?;
    crate::metadata::with_account_db(&app, generation, |c| {
        schema(c)?;
        let mut q = c
            .prepare("SELECT asin,rating,text FROM local_reviews ORDER BY asin")
            .map_err(|_| "Cannot read reviews")?;
        let rows = q
            .query_map([], |r| {
                Ok(Entry {
                    asin: r.get(0)?,
                    review: Review {
                        rating: r.get(1)?,
                        text: r.get(2)?,
                    },
                })
            })
            .map_err(|_| "Cannot read reviews")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Cannot read reviews".into())
    })
}
#[tauri::command]
/// Save/remove a local review against its expected original value. Native
/// validation and the account guard remain authoritative even if UI validation
/// passed.
pub fn save_review(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    review: Option<Review>,
    expected: Option<Review>,
) -> Result<(), String> {
    crate::local(&window)?;
    crate::metadata::with_account_db(&app, generation, |c| save(c, &asin, review, expected))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE books(asin TEXT PRIMARY KEY); INSERT INTO books VALUES('B012345678')",
        )
        .unwrap();
        schema(&c).unwrap();
        c
    }
    fn review(rating: Option<u8>, text: &str) -> Review {
        Review {
            rating,
            text: text.into(),
        }
    }
    #[test]
    fn zero_is_rated_and_edits_and_deletes_check_previous_value() {
        let c = db();
        let zero = review(Some(0), "");
        save(&c, "B012345678", Some(zero.clone()), None).unwrap();
        assert_eq!(get(&c, "B012345678").unwrap(), Some(zero.clone()));
        assert!(save(&c, "B012345678", Some(review(Some(50), "Changed")), None).is_err());
        assert!(save(&c, "B012345678", None, None).is_err());
        save(
            &c,
            "B012345678",
            Some(review(None, "Review only")),
            Some(zero),
        )
        .unwrap();
        save(&c, "B012345678", None, Some(review(None, "Review only"))).unwrap();
        assert!(get(&c, "B012345678").unwrap().is_none());
    }
    #[test]
    fn rejects_invalid_or_uncached_books_and_oversized_or_empty_reviews() {
        let c = db();
        for r in [
            review(Some(51), ""),
            review(None, "  "),
            review(Some(20), &"x".repeat(10001)),
        ] {
            assert!(save(&c, "B012345678", Some(r), None).is_err());
        }
        assert!(save(&c, "B987654321", Some(review(Some(45), "")), None).is_err());
        assert!(save(&c, "invalid", Some(review(Some(45), "")), None).is_err());
        save(&c, "B012345678", Some(review(Some(50), "Maximum")), None).unwrap();
    }
}
