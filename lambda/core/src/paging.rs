//! Paging, as `.stories/2-identity-api/spec.md` "Wire contract" ("Lists") says: every list is
//! `{"items": [...], "next": "<url or null>"}`, and `next` is the same route with an opaque
//! `?page=<token>`. This is the token for lists held in one file, such as friends: an offset.

use std::ops::Range;

use crate::problem::Problem;

/// The most items a page of objects holds.
pub const OFFSET_PAGE: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The items on this page.
    pub range: Range<usize>,
    /// The next page's path and query, or `None` on the last page.
    pub next: Option<String>,
}

/// The page of a list of `total` items that `token` names, `size` at a time, with `path` the
/// route's path for `next`. No token is the first page. A token that `next` never gave gets
/// `400`, and one past the end `404`.
pub fn offset_page(
    total: usize,
    token: Option<&str>,
    size: usize,
    path: &str,
) -> Result<Page, Problem> {
    let start = match token {
        None => 0,
        Some(t) => parse_offset(t)
            .ok_or_else(|| Problem::bad_request("?page= must be a token from a list's next"))?,
    };
    if start > 0 && start >= total {
        return Err(Problem::not_found("This page is past the end of the list"));
    }
    let end = total.min(start + size);
    let next = (end < total).then(|| format!("{path}?page={end}"));
    Ok(Page {
        range: start..end,
        next,
    })
}

/// Decimal, with no sign or leading zeros, as `next` writes it.
fn parse_offset(token: &str) -> Option<usize> {
    let canonical = !token.is_empty()
        && token.len() <= 6
        && token.bytes().all(|b| b.is_ascii_digit())
        && (token == "0" || !token.starts_with('0'));
    canonical.then(|| token.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/v1/identity/self/friends";

    #[test]
    fn pages_follow_next_to_the_end() {
        let first = offset_page(45, None, 20, PATH).unwrap();
        assert_eq!(first.range, 0..20);
        assert_eq!(
            first.next.as_deref(),
            Some("/v1/identity/self/friends?page=20")
        );
        let last = offset_page(45, Some("40"), 20, PATH).unwrap();
        assert_eq!((last.range, last.next), (40..45, None));
    }

    #[test]
    fn an_empty_list_has_one_empty_page() {
        let page = offset_page(0, None, 20, PATH).unwrap();
        assert_eq!((page.range, page.next), (0..0, None));
    }

    #[test]
    fn a_made_up_token_gets_400_and_one_past_the_end_404() {
        for token in ["", "x", "-1", "01", "+5", "9999999"] {
            assert_eq!(
                offset_page(45, Some(token), 20, PATH).unwrap_err().status,
                400,
                "{token:?}"
            );
        }
        assert_eq!(
            offset_page(45, Some("45"), 20, PATH).unwrap_err().status,
            404
        );
    }
}
