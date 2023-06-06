use askama::Template;
use derive_more::Constructor;

#[derive(Template, Constructor)]
#[template(path = "index.html")]
pub struct Index<'a> {
    pub title: &'a str,
}
