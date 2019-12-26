use crate::Statement;
use html5ever::{namespace_url, QualName};

#[macro_export]
macro_rules! qn {
	($thing:tt) => {{
		use $crate::html5ever::{self, namespace_url};
		html5ever::QualName::new(None, html5ever::ns!(), html5ever::local_name!($thing))
		}};
}

pub struct Rewrite {
	doc: debris::Document,
}

impl Rewrite {
	pub fn start(doc: debris::Document) -> Rewrite {
		Rewrite { doc }
	}

	pub fn export(self) -> Statement {
		Statement::HTML { html: self.doc.html() }
	}

	pub fn fix_hide(&mut self, mut f: impl FnMut(&mut ego_tree::NodeMut<scraper::Node>) -> bool) {
		Self::impl_fix_hide(self.doc.tree.tree.root_mut(), &mut f);
	}

	pub fn fix_override_csp(&mut self) {
		self.fix_traverse(|mut v| {
			let is_head =
				if let scraper::Node::Element(v) = v.value() { v.name() == "head" } else { false };
			if is_head {
				v.prepend(scraper::Node::Element(scraper::node::Element {
					name: qn!("meta"),
					id: None,
					classes: std::collections::HashSet::new(),
					attrs: vec![
						(qn!("http-equiv"), "Content-Security-Policy".into()),
						(qn!("content"), "default-src * 'unsafe-inline' 'unsafe-eval';".into()),
					]
					.into_iter()
					.collect(),
				}));
			}
		});
	}

	fn impl_fix_hide(
		mut v: ego_tree::NodeMut<scraper::Node>,
		f: &mut impl FnMut(&mut ego_tree::NodeMut<scraper::Node>) -> bool,
	) -> bool
	{
		let good_self = f(&mut v);
		let good_path =
			good_self || v.first_child().map(|kid| Self::impl_fix_hide(kid, f)).unwrap_or(false);
		let good_siblings =
			v.next_sibling().map(|sib| Self::impl_fix_hide(sib, f)).unwrap_or(false);
		if !good_path {
			if let scraper::Node::Element(v) = v.value() {
				add_style(v, "display: none !important;");
			}
		}
		good_path | good_siblings
	}

	pub fn fix_traverse(&mut self, mut f: impl FnMut(ego_tree::NodeMut<scraper::Node>)) {
		Self::impl_traversal(self.doc.tree.tree.root_mut(), &mut f);
	}

	fn impl_traversal(
		mut v: ego_tree::NodeMut<scraper::Node>,
		f: &mut impl FnMut(ego_tree::NodeMut<scraper::Node>),
	)
	{
		if let Some(kid) = v.first_child() {
			Self::impl_traversal(kid, f)
		}
		if let Some(sib) = v.next_sibling() {
			Self::impl_traversal(sib, f)
		}
		f(v);
	}
}

pub fn fix_url(v: &mut scraper::node::Element, key: QualName, scan: &str, prepend: &str) {
	if let Some(val) = v.attrs.get_mut(&key) {
		if val.starts_with(scan) {
			*val = format!("{}{}", prepend, val).into();
		}
	}
}

pub fn add_style(v: &mut scraper::node::Element, part: &str) {
	let old = v.attr("style").unwrap_or("").to_owned();
	v.attrs.insert(qn!("style"), format!("{}{}", old, part).into());
}

pub fn any_sibling(
	v: &mut ego_tree::NodeMut<scraper::Node>,
	mut f: impl FnMut(&mut ego_tree::NodeMut<scraper::Node>) -> bool,
) -> bool
{
	impl_any_sibling_prev(v, &mut f) || impl_any_sibling_next(v, &mut f)
}

fn impl_any_sibling_prev(
	v: &mut ego_tree::NodeMut<scraper::Node>,
	f: &mut impl FnMut(&mut ego_tree::NodeMut<scraper::Node>) -> bool,
) -> bool
{
	f(v) || v.prev_sibling().map_or(false, |mut u| impl_any_sibling_prev(&mut u, f))
}

fn impl_any_sibling_next(
	v: &mut ego_tree::NodeMut<scraper::Node>,
	f: &mut impl FnMut(&mut ego_tree::NodeMut<scraper::Node>) -> bool,
) -> bool
{
	f(v) || v.next_sibling().map_or(false, |mut u| impl_any_sibling_next(&mut u, f))
}
