#[test]
fn test_doc_book_toml() {
    version_sync::assert_contains_regex!(
        "book/book.toml",
        "title = \"Warpgrapher Book \\(v{version}\\)"
    );
}

#[test]
fn test_html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}

#[test]
fn test_readme_deps() {
    version_sync::assert_markdown_deps_updated!("README.md");
}

#[test]
fn test_quickstart_version() {
    version_sync::assert_markdown_deps_updated!("book/src/warpgrapher/quickstart.md");
}

#[test]
fn test_databases_version() {
    version_sync::assert_markdown_deps_updated!("book/src/warpgrapher/databases.md");
}
