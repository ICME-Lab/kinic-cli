//! Shared helpers for cross-memory search target selection and result folding.
//! Where: reused by CLI search and TUI live-search code.
//! What: normalizes searchable memory ids, search-hit rows, and partial-failure folding.
//! Why: keep all-memory search behavior aligned without sharing UI-specific orchestration.

use std::cmp::Ordering;

use crate::clients::launcher::State;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub memory_id: String,
    pub score: f32,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FoldedSearchBatch<T> {
    pub items: Vec<T>,
    pub failed_memory_ids: Vec<String>,
}

pub fn searchable_memory_id_from_state(state: State) -> Option<String> {
    match state {
        State::Installation(principal, _)
        | State::SettingUp(principal)
        | State::Running(principal) => Some(principal.to_text()),
        _ => None,
    }
}

pub fn collect_searchable_memory_ids<I>(ids: I, empty_message: &str) -> Result<Vec<String>, String>
where
    I: IntoIterator<Item = Option<String>>,
{
    let collected = ids.into_iter().flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        Err(empty_message.to_string())
    } else {
        Ok(collected)
    }
}

pub fn sort_search_hits(rows: &mut [SearchHit]) {
    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.memory_id.cmp(&right.memory_id))
            .then_with(|| left.payload.cmp(&right.payload))
    });
}

pub fn fold_search_batches<T, E>(
    target_count: usize,
    results: Vec<(String, Result<Vec<T>, E>)>,
    join_errors: Vec<String>,
    join_error_memory_id: &str,
    all_failed_message: &str,
) -> Result<FoldedSearchBatch<T>, String>
where
    E: ToString,
{
    let mut items = Vec::new();
    let mut failed_memory_ids = Vec::new();
    let mut first_error = None;

    for (memory_id, result) in results {
        match result {
            Ok(mut next_items) => items.append(&mut next_items),
            Err(error) => {
                failed_memory_ids.push(memory_id);
                if first_error.is_none() {
                    first_error = Some(error.to_string());
                }
            }
        }
    }

    for error in join_errors {
        failed_memory_ids.push(join_error_memory_id.to_string());
        if first_error.is_none() {
            first_error = Some(error);
        }
    }

    if target_count > 0 && failed_memory_ids.len() == target_count {
        return Err(first_error.unwrap_or_else(|| all_failed_message.to_string()));
    }

    Ok(FoldedSearchBatch {
        items,
        failed_memory_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn searchable_memory_id_from_state_keeps_searchable_variants_only() {
        let running = searchable_memory_id_from_state(State::Running(Principal::anonymous()));
        let empty = searchable_memory_id_from_state(State::Empty("none".to_string()));

        assert_eq!(running.as_deref(), Some("2vxsx-fae"));
        assert_eq!(empty, None);
    }

    #[test]
    fn collect_searchable_memory_ids_rejects_empty_inputs() {
        let error =
            collect_searchable_memory_ids(Vec::<Option<String>>::new(), "no memories").unwrap_err();
        assert_eq!(error, "no memories");
    }

    #[test]
    fn sort_search_hits_orders_by_score_desc_then_memory_id() {
        let mut rows = vec![
            SearchHit {
                memory_id: "bbbbb-bb".to_string(),
                score: 0.2,
                payload: "beta".to_string(),
            },
            SearchHit {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "alpha".to_string(),
            },
            SearchHit {
                memory_id: "ccccc-cc".to_string(),
                score: 0.9,
                payload: "gamma".to_string(),
            },
        ];

        sort_search_hits(&mut rows);

        assert_eq!(rows[0].memory_id, "aaaaa-aa");
        assert_eq!(rows[1].memory_id, "ccccc-cc");
        assert_eq!(rows[2].memory_id, "bbbbb-bb");
    }

    #[test]
    fn fold_search_batches_keeps_partial_success_and_failed_ids() {
        let batch = fold_search_batches(
            2,
            vec![
                (
                    "aaaaa-aa".to_string(),
                    Ok(vec![SearchHit {
                        memory_id: "aaaaa-aa".to_string(),
                        score: 0.5,
                        payload: "alpha".to_string(),
                    }]),
                ),
                ("bbbbb-bb".to_string(), Err("boom")),
            ],
            Vec::new(),
            "join-error",
            "all failed",
        )
        .expect("partial success should survive");

        assert_eq!(batch.items.len(), 1);
        assert_eq!(batch.failed_memory_ids, vec!["bbbbb-bb".to_string()]);
    }

    #[test]
    fn fold_search_batches_errors_when_everything_fails() {
        let error = fold_search_batches::<SearchHit, _>(
            1,
            vec![("aaaaa-aa".to_string(), Err("boom"))],
            Vec::new(),
            "join-error",
            "all failed",
        )
        .unwrap_err();

        assert_eq!(error, "boom");
    }
}
