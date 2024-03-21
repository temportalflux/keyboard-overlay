use itertools::Itertools;
use std::collections::BTreeMap;
use yew::AttrValue;

#[derive(Clone, PartialEq, Default)]
pub struct Style(BTreeMap<String, String>);

impl<I, K, V> From<I> for Style
where
	I: IntoIterator<Item = (K, V)>,
	K: ToString,
	V: ToString,
{
	fn from(value: I) -> Self {
		Self(value.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
	}
}

impl Style {
	pub fn with(mut self, key: impl ToString, value: impl ToString) -> Self {
		self.insert(key, value);
		self
	}

	pub fn insert(&mut self, key: impl ToString, value: impl ToString) -> Option<String> {
		self.0.insert(key.to_string(), value.to_string())
	}

	pub fn extend_with(mut self, other: Self) -> Self {
		self.extend(other);
		self
	}

	pub fn extend(&mut self, other: Self) {
		for (key, value) in other.0 {
			let _ = self.insert(key, value);
		}
	}
}

impl std::ops::Add<Style> for &Style {
	type Output = Style;

	fn add(self, rhs: Style) -> Self::Output {
		self.clone().extend_with(rhs)
	}
}
impl std::ops::Add<&Style> for Style {
	type Output = Style;

	fn add(self, rhs: &Style) -> Self::Output {
		self.extend_with(rhs.clone())
	}
}

impl yew::html::IntoPropValue<Option<AttrValue>> for Style {
	fn into_prop_value(self) -> Option<AttrValue> {
		if self.0.is_empty() {
			return None;
		}
		let entries = self.0.into_iter();
		let properties = entries.map(|(key, value)| format!("{key}: {value};"));
		#[allow(unstable_name_collisions)]
		let properties = properties.intersperse(" ".to_owned());
		Some(properties.collect::<String>().into())
	}
}
