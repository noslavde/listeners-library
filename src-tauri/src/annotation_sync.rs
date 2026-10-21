//! Explicit experimental annotation reconciliation. Stable remote IDs, a
//! three-way baseline and durable pending-write intents preserve local/cloud
//! conflicts and prevent blind retries after uncertain sends; no automatic
//! cloud deletion is performed.

//! Conservative sync: stable IDs, three-way merges and verified individual writes.
use crate::highlights::Passage;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Emitter, Manager};
pub struct Jobs(pub tokio::sync::Mutex<()>);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Remote {
    id: String,
    passage: Passage,
    wire_end: Option<u64>,
    #[serde(default)]
    write_safe: bool,
}
#[derive(Default, Serialize)]
pub struct Receipt {
    imported: usize,
    updated: usize,
    conflicts: usize,
    missing: usize,
    message: String,
}
/// Create local link/baseline and uncertain-attempt tables. Local deletion
/// hides a linked remote ID instead of deleting cloud data.
pub fn schema(c: &Connection) -> Result<(), String> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS annotation_links(asin TEXT NOT NULL,remote_id TEXT NOT NULL,note_id INTEGER UNIQUE,base_json TEXT NOT NULL,remote_json TEXT NOT NULL,state TEXT NOT NULL,PRIMARY KEY(asin,remote_id));CREATE TABLE IF NOT EXISTS annotation_attempts(note_id INTEGER PRIMARY KEY,asin TEXT NOT NULL,token TEXT NOT NULL,desired_json TEXT NOT NULL);CREATE TRIGGER IF NOT EXISTS hide_deleted_annotations AFTER DELETE ON notes BEGIN UPDATE annotation_links SET note_id=NULL,state='hidden' WHERE note_id=old.rowid;DELETE FROM annotation_attempts WHERE note_id=old.rowid;END;").map_err(|_|"Cannot prepare encrypted annotation sync".into())
}
fn encode<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|_| "Cannot encode annotation".to_string())
}
fn ms(v: &Value) -> Result<u64, String> {
    let n = v
        .as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        .ok_or("Invalid annotation timestamp")?;
    if n >= 31_536_000_000 {
        return Err("Annotation position out of range".into());
    }
    Ok(n)
}
fn parse(v: Value, asin: &str) -> Result<Vec<Remote>, String> {
    let p = &v["payload"];
    if p["type"] != "AUDI" || p["key"] != asin {
        return Err("Annotation response belongs to a different book; nothing imported".into());
    }
    let rows = p["records"]
        .as_array()
        .filter(|r| r.len() <= 10000)
        .ok_or("Unsupported annotation list")?;
    let mut out = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for r in rows {
        let kind = match r["type"].as_str() {
            Some("audible.last_heard") => continue,
            Some("audible.bookmark") => "bookmark",
            Some("audible.note") => "note",
            Some("audible.clip") => "clip",
            _ => return Err("Unknown annotation type; nothing imported or removed".into()),
        };
        let id = r["annotationId"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control))
            .ok_or("Annotation has no stable ID; nothing imported")?
            .to_owned();
        if !ids.insert(id.clone()) {
            return Err("Duplicate annotation IDs; nothing imported".into());
        }
        let begin = ms(&r["startPosition"])?;
        let end = if kind == "bookmark" {
            None
        } else {
            Some(ms(&r["endPosition"])?)
        };
        let text = match kind {
            "note" => r["text"].as_str().unwrap_or("").to_owned(),
            "clip" => {
                let title = r["metadata"]["title"].as_str().unwrap_or("");
                let note = r["metadata"]["note"].as_str().unwrap_or("");
                if title.is_empty() {
                    note.to_owned()
                } else if note.is_empty() {
                    title.to_owned()
                } else {
                    format!("{title}\n{note}")
                }
            }
            _ => String::new(),
        };
        let passage = Passage {
            kind: kind.into(),
            position: begin as f64 / 1000.,
            end_position: if kind == "clip" {
                end.map(|e| e as f64 / 1000.)
            } else {
                None
            },
            text: text.trim().to_owned(),
        };
        crate::highlights::valid(&passage)
            .map_err(|_| "An Audible annotation cannot be represented safely; nothing imported")?;
        out.push(Remote {
            id,
            passage,
            wire_end: end,
            write_safe: if kind == "clip" {
                r.get("metadata")
                    .and_then(Value::as_object)
                    .is_some_and(|m| {
                        m.iter()
                            .all(|(k, v)| matches!(k.as_str(), "title" | "note") && v.is_string())
                    })
            } else {
                r.get("metadata").is_none()
            },
        });
    }
    Ok(out)
}
fn key(p: &Passage) -> (String, u64, u64) {
    (
        p.kind.clone(),
        (p.position * 1000.).round() as u64,
        (p.end_position.unwrap_or(p.position) * 1000.).round() as u64,
    )
}
fn matches(r: &Remote, p: &Passage) -> bool {
    r.passage == *p
}
fn linked(c: &Connection, asin: &str, r: &Remote, id: i64) -> Result<(), String> {
    c.execute("INSERT INTO annotation_links(asin,remote_id,note_id,base_json,remote_json,state) VALUES(?1,?2,?3,?4,?5,'synced') ON CONFLICT(asin,remote_id) DO UPDATE SET note_id=excluded.note_id,base_json=excluded.base_json,remote_json=excluded.remote_json,state='synced'",params![asin,r.id,id,encode(&r.passage)?,encode(r)?]).map_err(|_|"Cannot link Audible annotation")?;
    c.execute(
        "DELETE FROM annotation_attempts WHERE note_id=?1 AND desired_json=?2",
        params![id, encode(&r.passage)?],
    )
    .map_err(|_| "Cannot finish annotation verification")?;
    Ok(())
}
/// Reconcile baseline/local/remote values atomically. Matching position alone is
/// insufficient identity: only stable links or unique exact content may merge.
/// Roll back the whole batch if one row cannot be interpreted safely.
fn merge(c: &Connection, asin: &str, rows: &[Remote]) -> Result<Receipt, String> {
    crate::notes::schema(c)?;
    c.execute_batch("SAVEPOINT merge_annotations")
        .map_err(|_| "Cannot begin annotation merge")?;
    let result = (|| {
        let mut receipt = Receipt::default();
        let mut q=c.prepare("SELECT n.rowid FROM notes n LEFT JOIN annotation_links l ON l.note_id=n.rowid WHERE n.asin=?1 AND l.note_id IS NULL").map_err(|_|"Cannot inspect local annotations")?;
        let ids = q
            .query_map([asin], |r| r.get::<_, i64>(0))
            .map_err(|_| "Cannot inspect local annotations")?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Cannot inspect local annotations")?;
        drop(q);
        let mut local_matches = std::collections::HashMap::<String, Vec<i64>>::new();
        for id in ids {
            local_matches
                .entry(encode(&crate::highlights::get(c, asin, id)?)?)
                .or_default()
                .push(id);
        }
        for r in rows {
            let prior:Option<(Option<i64>,String,String)>=c.query_row("SELECT note_id,base_json,state FROM annotation_links WHERE asin=?1 AND remote_id=?2",params![asin,r.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|"Cannot read annotation links")?;
            if let Some((id, base, state)) = prior {
                if state == "hidden" {
                    continue;
                }
                let Some(id) = id else {
                    continue;
                };
                let local = crate::highlights::get(c, asin, id)?;
                let base: Passage =
                    serde_json::from_str(&base).map_err(|_| "Unreadable annotation checkpoint")?;
                if local == r.passage {
                    linked(c, asin, r, id)?;
                } else if local == base {
                    crate::highlights::save(c, asin, &r.passage, Some((id, &local)))?;
                    linked(c, asin, r, id)?;
                    receipt.updated += 1;
                } else if base != r.passage {
                    c.execute("UPDATE annotation_links SET remote_json=?1,state='conflict' WHERE asin=?2 AND remote_id=?3",params![encode(r)?,asin,r.id]).map_err(|_|"Cannot preserve conflicting annotation")?;
                    receipt.conflicts += 1;
                } else {
                    c.execute("UPDATE annotation_links SET state='synced',remote_json=?1 WHERE asin=?2 AND remote_id=?3",params![encode(r)?,asin,r.id]).map_err(|_|"Cannot refresh annotation")?;
                }
            } else {
                // Adopt only a unique exact pending/local match; never match on
                // position alone, which could replace another user's text.
                let candidates = local_matches.entry(encode(&r.passage)?).or_default();
                let id = if candidates.len() == 1 {
                    candidates.pop().unwrap()
                } else {
                    crate::highlights::save(c, asin, &r.passage, None)?;
                    receipt.imported += 1;
                    c.last_insert_rowid()
                };
                linked(c, asin, r, id)?;
                c.execute(
                    "DELETE FROM annotation_attempts WHERE note_id=?1 AND desired_json=?2",
                    params![id, encode(&r.passage)?],
                )
                .map_err(|_| "Cannot finish annotation verification")?;
            }
        }
        let seen = rows
            .iter()
            .map(|r| r.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let mut q=c.prepare("SELECT remote_id FROM annotation_links WHERE asin=?1 AND note_id IS NOT NULL AND state!='hidden'").map_err(|_|"Cannot check remote deletions")?;
        let all = q
            .query_map([asin], |r| r.get::<_, String>(0))
            .map_err(|_| "Cannot check remote deletions")?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Cannot check remote deletions")?;
        drop(q);
        for id in all {
            if !seen.contains(id.as_str()) {
                c.execute("UPDATE annotation_links SET state='remote-missing' WHERE asin=?1 AND remote_id=?2",params![asin,id]).map_err(|_|"Cannot preserve missing annotation")?;
                receipt.missing += 1;
            }
        }
        Ok(receipt)
    })();
    if result.is_err() {
        let _ = c.execute_batch("ROLLBACK TO merge_annotations");
    }
    c.execute_batch("RELEASE merge_annotations")
        .map_err(|_| "Cannot finish annotation merge")?;
    result
}
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn wire(asin: &str, p: &Passage, date: &str) -> Result<String, String> {
    crate::highlights::valid(p)?;
    if !crate::valid_asin(asin) {
        return Err("Invalid book".into());
    }
    if p.kind == "bookmark" && !p.text.is_empty() {
        return Err(
            "Audible bookmarks cannot carry this label. Keep it local or create a note instead."
                .into(),
        );
    }
    if p.kind == "clip" && !(5.0..=44.0).contains(&(p.end_position.unwrap_or(0.) - p.position)) {
        return Err("For mobile compatibility, send clips between 5 and 44 seconds; longer clips stay local.".into());
    }
    if p.text.chars().any(|c| {
        (c < ' ' && !matches!(c, '\n' | '\t' | '\r')) || matches!(c, '\u{fffe}' | '\u{ffff}')
    }) {
        return Err("Annotation text contains unsupported control characters".into());
    }
    let (_, start, end) = key(p);
    let date = xml_escape(date);
    let content=match p.kind.as_str(){
        "bookmark"=>format!("<bookmark action=\"create\" begin=\"{start}\" timestamp=\"{date}\"/>"),
        "note"=>format!("<note action=\"create\" begin=\"{start}\" end=\"{start}\" timestamp=\"{date}\">{}</note>",xml_escape(&p.text)),
        "clip"=>{let (title,note)=p.text.split_once('\n').unwrap_or((&p.text,""));let meta=encode(&serde_json::json!({"title":title,"note":note}))?;let meta=meta.replace("]]>","]]]]><![CDATA[>");format!("<clip action=\"create\" begin=\"{start}\" end=\"{end}\" timestamp=\"{date}\"><metadata><![CDATA[{meta}]]></metadata></clip>")},
        _=>return Err("Unsupported annotation kind".into())
    };
    Ok(format!("<annotations version=\"1.0\" timestamp=\"{date}\"><book key=\"{asin}\" type=\"AUDI\" guid=\"_LATEST_\">{content}</book></annotations>"))
}
async fn read(
    app: &tauri::AppHandle,
    generation: u64,
    asin: &str,
) -> Result<Option<Vec<Remote>>, String> {
    crate::metadata::annotations::request(app, generation, asin, None)
        .await?
        .map(|v| parse(v, asin))
        .transpose()
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit_to("main", "notes-changed", ());
}
#[tauri::command]
pub async fn annotation_sync_status(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
) -> Result<bool, String> {
    crate::local(&window)?;
    crate::metadata::annotations::available(&app, generation).await
}
#[tauri::command]
/// Explicitly read and reconcile one owned title. Unknown/ambiguous remote
/// responses must preserve local content rather than act as authoritative
/// deletion.
pub async fn pull_annotations(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
) -> Result<Receipt, String> {
    crate::local(&window)?;
    let jobs = app.state::<Jobs>();
    let _job = jobs.0.lock().await;
    crate::metadata::with_account_db(&app, generation, |c| {
        if !c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM books WHERE asin=?1)",
                [&asin],
                |r| r.get::<_, bool>(0),
            )
            .map_err(|_| "Cannot check library")?
        {
            return Err("Choose a library book".into());
        }
        Ok(())
    })?;
    let Some(rows) = read(&app, generation, &asin).await? else {
        return Ok(Receipt{message:"Audible returned no sidecar (HTTP 404). This may mean no annotations or an unsupported request. Nothing was removed. Create a mobile test bookmark, then refresh.".into(),..Default::default()});
    };
    let mut receipt =
        crate::metadata::with_account_db(&app, generation, |c| merge(c, &asin, &rows))?;
    receipt.message=format!("Audible read complete: {} imported, {} updated, {} conflicts, {} missing remotely. No local items were deleted.",receipt.imported,receipt.updated,receipt.conflicts,receipt.missing);
    changed(&app);
    Ok(receipt)
}
#[tauri::command]
/// Preflight against fresh remote state, persist a write intent, send one
/// eligible item and verify by read-back. An uncertain send blocks blind retry.
pub async fn push_annotation(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    id: i64,
    expected: Passage,
) -> Result<String, String> {
    crate::local(&window)?;
    let jobs = app.state::<Jobs>();
    let _job = jobs.0.lock().await;
    crate::metadata::with_account_db(&app, generation, |c| {
        if crate::highlights::get(c, &asin, id)? != expected {
            return Err("The local item changed; refresh before sending".into());
        }
        Ok(())
    })?;
    let body = zeroize::Zeroizing::new(wire(
        &asin,
        &expected,
        &crate::metadata::annotations::timestamp(),
    )?);
    let rows=read(&app,generation,&asin).await?.ok_or("First create a disposable bookmark on mobile and successfully refresh this book. HTTP 404 is not a verified empty list; no write sent.")?;
    let token = format!("{:032x}", rand::random::<u128>());
    let should_send = crate::metadata::with_account_db(&app, generation, |c| {
        crate::notes::schema(c)?;
        if crate::highlights::get(c, &asin, id)? != expected {
            return Err("The local item changed; refresh before sending".into());
        }
        let candidates = rows
            .iter()
            .filter(|r| key(&r.passage) == key(&expected))
            .collect::<Vec<_>>();
        if candidates.len() == 1 && matches(candidates[0], &expected) {
            linked(c, &asin, candidates[0], id)?;
            c.execute("DELETE FROM annotation_attempts WHERE note_id=?1", [id])
                .map_err(|_| "Cannot finish verification")?;
            return Ok(false);
        }
        let base:Option<(String,String,String)>=c.query_row("SELECT remote_id,base_json,state FROM annotation_links WHERE note_id=?1 AND asin=?2",params![id,asin],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|"Cannot check previous sync")?;
        if let Some((remote_id, base, state)) = base {
            if state == "conflict" || state == "remote-missing" {
                return Err("Resolve the remote conflict before sending".into());
            }
            let base: Passage =
                serde_json::from_str(&base).map_err(|_| "Unreadable sync checkpoint")?;
            if candidates.len() != 1
                || candidates[0].id != remote_id
                || candidates[0].passage != base
                || key(&base) != key(&expected)
            {
                return Err("Audible changed or the saved times moved. Refresh and resolve the conflict; no write sent.".into());
            }
            if !candidates[0].write_safe {
                return Err(
                    "This Audible item has metadata not supported for editing; kept local".into(),
                );
            }
            if expected.kind == "note" && candidates[0].wire_end != Some(key(&expected).1) {
                return Err(
                    "This Audible note has a range not supported for writing; kept local".into(),
                );
            }
        } else if !candidates.is_empty() {
            return Err("Another Audible item occupies these times. Refresh and edit the imported item instead; no overwrite sent.".into());
        }
        let unknown: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM annotation_attempts WHERE note_id=?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| "Cannot check pending write")?;
        if unknown {
            return Err("A previous send has an unknown outcome. Refresh and check mobile. Automatic retries are blocked to avoid duplicates.".into());
        }
        c.execute(
            "INSERT INTO annotation_attempts(note_id,asin,token,desired_json) VALUES(?1,?2,?3,?4)",
            params![id, asin, token, encode(&expected)?],
        )
        .map_err(|_| "Cannot save write intent; no request sent")?;
        Ok(true)
    })?;
    if !should_send {
        changed(&app);
        return Ok(
            "Verified on Audible; no duplicate was sent. Confirm visibility on mobile.".into(),
        );
    }
    // Any ambiguous network outcome leaves an encrypted durable intent. A
    // subsequent pull can verify it; no blind create retries are permitted.
    let sent = crate::metadata::annotations::request(&app, generation, &asin, Some(&body)).await;
    changed(&app);
    sent?;
    let mut found = None;
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        if let Some(rows) = read(&app, generation, &asin).await? {
            let matching = rows
                .into_iter()
                .filter(|r| matches(r, &expected))
                .collect::<Vec<_>>();
            if matching.len() == 1 {
                found = matching.into_iter().next();
                break;
            }
        }
    }
    let Some(remote) = found else {
        return Err("Audible accepted the request but read-back is not verified. Check mobile and refresh; no automatic retry will be sent.".into());
    };
    crate::metadata::with_account_db(&app, generation, |c| {
        let present: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM annotation_attempts WHERE note_id=?1 AND token=?2)",
                params![id, token],
                |r| r.get(0),
            )
            .map_err(|_| "Cannot verify write intent")?;
        if !present {
            return Err(
                "The local item was removed while sending; refresh Audible to inspect the result"
                    .into(),
            );
        }
        linked(c, &asin, &remote, id)?;
        c.execute(
            "DELETE FROM annotation_attempts WHERE note_id=?1 AND token=?2",
            params![id, token],
        )
        .map_err(|_| "Cannot finish annotation verification")?;
        Ok(())
    })?;
    changed(&app);
    Ok("Verified by Audible read-back. Please confirm it appears on mobile.".into())
}
#[tauri::command]
/// Apply the reviewed local/cloud conflict choice only if both previews still
/// match. Adopting Audible keeps a separate PC copy; keeping local detaches the
/// old remote link.
pub fn resolve_annotation(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    id: i64,
    expected: Passage,
    use_audible: bool,
    expected_remote: Option<Passage>,
    expected_state: String,
) -> Result<(), String> {
    crate::local(&window)?;
    crate::metadata::with_account_db(&app, generation, |c| {
        crate::notes::schema(c)?;
        if crate::highlights::get(c, &asin, id)? != expected {
            return Err("Local item changed; review again".into());
        }
        let (raw, state): (String, String) = c
            .query_row(
                "SELECT remote_json,state FROM annotation_links WHERE note_id=?1 AND asin=?2",
                params![id, asin],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| "Refresh this item's Audible state first")?;
        let reviewed: Remote = serde_json::from_str(&raw).map_err(|_| "Unreadable conflict")?;
        if state != expected_state || expected_remote.as_ref() != Some(&reviewed.passage) {
            return Err(
                "The Audible version changed since your preview. Refresh and review again.".into(),
            );
        }
        if !matches!(state.as_str(), "conflict" | "remote-missing") {
            return Err("Refresh the conflict before resolving".into());
        }
        c.execute_batch("SAVEPOINT resolve_annotation")
            .map_err(|_| "Cannot resolve annotation")?;
        let result = (|| {
            if use_audible {
                if state == "remote-missing" {
                    return Err("Audible no longer returned this item. Keep it locally; no automatic deletion is performed.".into());
                }
                let remote: Remote =
                    serde_json::from_str(&raw).map_err(|_| "Unreadable conflict")?;
                // Retain the user's edited copy before accepting the other side.
                crate::highlights::save(c, &asin, &expected, None)?;
                crate::highlights::save(c, &asin, &remote.passage, Some((id, &expected)))?;
                linked(c, &asin, &remote, id)?;
            } else {
                c.execute(
                    "UPDATE annotation_links SET note_id=NULL,state='hidden' WHERE note_id=?1",
                    [id],
                )
                .map_err(|_| "Cannot keep local copy")?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = c.execute_batch("ROLLBACK TO resolve_annotation");
        }
        c.execute_batch("RELEASE resolve_annotation")
            .map_err(|_| "Cannot finish conflict resolution")?;
        result
    })?;
    changed(&app);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE notes(asin TEXT,position REAL,text TEXT);CREATE TABLE books(asin TEXT,metadata TEXT);INSERT INTO books VALUES('B012345678','{\"duration\":600}');").unwrap();
        crate::notes::schema(&c).unwrap();
        c
    }
    fn row(text: &str) -> Remote {
        Remote {
            id: "cloud-1".into(),
            passage: Passage {
                kind: "note".into(),
                position: 42.,
                end_position: None,
                text: text.into(),
            },
            wire_end: Some(42000),
            write_safe: true,
        }
    }
    #[test]
    fn imports_once_and_uses_three_way_conflicts() {
        let c = db();
        assert_eq!(
            merge(&c, "B012345678", &[row("mobile")]).unwrap().imported,
            1
        );
        assert_eq!(
            merge(&c, "B012345678", &[row("mobile")]).unwrap().imported,
            0
        );
        let old = crate::highlights::get(&c, "B012345678", 1).unwrap();
        let mut edit = old.clone();
        edit.text = "PC edit".into();
        crate::highlights::save(&c, "B012345678", &edit, Some((1, &old))).unwrap();
        assert_eq!(
            merge(&c, "B012345678", &[row("mobile edit")])
                .unwrap()
                .conflicts,
            1
        );
        assert_eq!(
            crate::highlights::get(&c, "B012345678", 1).unwrap().text,
            "PC edit"
        );
    }
    #[test]
    fn clean_remote_edits_update_but_remote_deletion_never_erases() {
        let c = db();
        merge(&c, "B012345678", &[row("one")]).unwrap();
        assert_eq!(merge(&c, "B012345678", &[row("two")]).unwrap().updated, 1);
        assert_eq!(merge(&c, "B012345678", &[]).unwrap().missing, 1);
        assert_eq!(
            crate::highlights::get(&c, "B012345678", 1).unwrap().text,
            "two"
        );
    }
    #[test]
    fn local_deletion_stays_hidden_and_id_reuse_does_not_inherit_link() {
        let c = db();
        merge(&c, "B012345678", &[row("one")]).unwrap();
        c.execute("DELETE FROM notes WHERE rowid=1", []).unwrap();
        crate::highlights::save(&c, "B012345678", &row("local").passage, None).unwrap();
        merge(&c, "B012345678", &[row("one")]).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM annotation_links WHERE note_id IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
    #[test]
    fn rejects_wrong_book_and_bad_units() {
        let v = json!({"payload":{"type":"AUDI","key":"B987654321","records":[]}});
        assert!(parse(v, "B012345678").is_err());
        assert_eq!(ms(&json!("42000")).unwrap(), 42000);
        assert!(ms(&json!(-1)).is_err());
    }
    #[test]
    fn unknown_clip_metadata_is_imported_read_only_and_duplicate_ids_are_rejected() {
        let record = json!({"annotationId":"synthetic-cloud-id","type":"audible.clip","startPosition":"42000","endPosition":"72000","metadata":{"title":"repeat","note":"repeat","future_field":"preserve remotely"}});
        let v = json!({"payload":{"type":"AUDI","key":"B012345678","records":[record.clone()]}});
        let rows = parse(v, "B012345678").unwrap();
        assert!(!rows[0].write_safe);
        assert_eq!(rows[0].passage.text, "repeat\nrepeat");
        let v =
            json!({"payload":{"type":"AUDI","key":"B012345678","records":[record.clone(),record]}});
        assert!(parse(v, "B012345678").is_err());
    }
    #[test]
    fn xml_is_escaped_and_never_writes_progress() {
        let mut p = row("<& special").passage;
        let body = wire("B012345678", &p, "2026-10-20T00:00:00Z").unwrap();
        assert!(body.contains("&lt;&amp; special"));
        assert!(!body.contains("last_heard"));
        p.kind = "clip".into();
        p.end_position = Some(72.);
        p.text = "title\n]]> & note".into();
        let body = wire("B012345678", &p, "date").unwrap();
        assert!(body.contains("]]]]><![CDATA[>"));
        p.end_position = Some(100.);
        assert!(wire("B012345678", &p, "date").is_err());
    }
    #[test]
    fn failed_merge_rolls_back_all_records_and_links() {
        let c = db();
        let first = row("valid");
        let mut invalid = row("past end");
        invalid.id = "cloud-2".into();
        invalid.passage.position = 700.;
        assert!(merge(&c, "B012345678", &[first, invalid]).is_err());
        for table in ["notes", "annotation_links"] {
            assert_eq!(
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
    #[test]
    fn later_pull_verifies_an_uncertain_text_edit() {
        let c = db();
        merge(&c, "B012345678", &[row("old")]).unwrap();
        let old = crate::highlights::get(&c, "B012345678", 1).unwrap();
        let desired = row("new").passage;
        crate::highlights::save(&c, "B012345678", &desired, Some((1, &old))).unwrap();
        c.execute(
            "INSERT INTO annotation_attempts VALUES(1,'B012345678','synthetic-ticket',?1)",
            [encode(&desired).unwrap()],
        )
        .unwrap();
        merge(&c, "B012345678", &[row("new")]).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM annotation_attempts", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
