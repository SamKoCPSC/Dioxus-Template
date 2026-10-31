fn main() {
    println!("cargo:rerun-if-changed=app.json");
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("app.json").expect("app.json must exist"))
            .expect("valid app.json");
    let name = value["name"].as_str().expect("app.name");
    let slug = value["slug"].as_str().expect("app.slug");
    assert!(
        !name.trim().is_empty()
            && name.chars().count() <= 80
            && !name.chars().any(char::is_control),
        "name must be 1-80 printable characters"
    );
    assert!(
        (3..=48).contains(&slug.len())
            && slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !slug.starts_with('-')
            && !slug.ends_with('-'),
        "slug must be 3-48 lowercase letters/digits/hyphens"
    );
    println!("cargo:rustc-env=APP_NAME={name}");
    println!("cargo:rustc-env=APP_SLUG={slug}");
}
