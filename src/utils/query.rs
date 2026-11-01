//! Decode raw URL query fields once, after splitting their encoded separators.
//! Dioxus 0.7.9's typed query parsing decodes too early for nested return URLs.
pub fn value(path: &str, name: &str) -> Option<String> {
    let query = path.split_once('?').map_or("", |(_, query)| query);
    let query = query.split('#').next().unwrap_or_default();
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

#[cfg(test)]
mod tests {
    use super::value;
    #[test]
    fn search_fields_preserve_literal_separators_and_decode_once() {
        assert_eq!(
            value("/search?q=100%25_+%26+ideas+%2F+caf%C3%A9", "q").as_deref(),
            Some("100%_ & ideas / café")
        );
        assert_eq!(
            value("/search?q=C%2B%2B%23notes&other=ignored", "q").as_deref(),
            Some("C++#notes")
        );
        assert_eq!(value("/search?q=%2526", "q").as_deref(), Some("%26"));
        assert_eq!(
            value("/search?q=first&q=second#results", "q").as_deref(),
            Some("first")
        );
        assert_eq!(value("/search", "q"), None);
    }
    #[test]
    fn sign_in_preserves_escaped_query_inside_a_valid_return_path() {
        let target = "/search?q=100%25_%20%26%20ideas";
        let encoded = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("redirect_uri", target)
            .finish();
        let parsed = value(&format!("/login?error=retry&{encoded}"), "redirect_uri").unwrap();
        assert_eq!(parsed, target);
        assert_eq!(
            crate::auth::redirect::normalize_same_origin_redirect(&parsed).as_deref(),
            Some(target)
        );
    }
}
