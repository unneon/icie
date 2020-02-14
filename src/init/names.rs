use crate::{
	dir, init::{PathDialog, ASK_FOR_PATH, PROJECT_NAME_TEMPLATE}, interpolation::Interpolation, util::path::Path
};
use evscode::R;
use std::{fmt, str::FromStr};
use unijudge::TaskDetails;

/// Default contest directory name. This key uses special syntax to allow using dynamic content,
/// like contest ids. Variables task.id and task.title are not available in this context. See "Icie
/// Init Project name template" for details.
#[evscode::config]
static CONTEST: evscode::Config<Interpolation<ContestVariable>> =
	"{contest.title case.kebab}".parse().unwrap();

/// Default task directory name, when created as a part of a contest. This key uses special syntax
/// to allow using dynamic content, like task titles. Variable contest.title is not available in
/// this context. See "Icie Init Project name template" for details.
#[evscode::config]
static CONTEST_TASK: evscode::Config<Interpolation<ContestTaskVariable>> =
	"{task.symbol case.upper}-{task.name case.kebab}".parse().unwrap();

pub async fn design_task_name(root: &Path, meta: Option<&TaskDetails>) -> R<Path> {
	let variables = Mapping {
		task_id: meta.as_ref().map(|meta| meta.id.clone()),
		task_title: meta.as_ref().map(|meta| meta.title.clone()),
		contest_id: meta.as_ref().map(|meta| meta.contest_id.clone()),
		contest_title: None,
		site_short: meta.as_ref().map(|meta| meta.site_short.clone()),
	};
	let (codename, all_good) = PROJECT_NAME_TEMPLATE.get().interpolate(&variables);
	let config_strategy = ASK_FOR_PATH.get();
	let strategy = match (config_strategy, all_good) {
		(_, false) => PathDialog::InputBox,
		(s, true) => s,
	};
	strategy.query(root, &codename).await
}

pub async fn design_contest_name(
	contest_id: String,
	contest_title: String,
	site_short: &'static str,
) -> R<Path>
{
	let variables = Mapping {
		task_id: None,
		task_title: None,
		contest_id: Some(contest_id.to_owned()),
		contest_title: Some(contest_title.to_owned()),
		site_short: Some(site_short.to_owned()),
	};
	let (codename, all_good) = CONTEST.get().interpolate(&variables);
	let config_strategy = ASK_FOR_PATH.get();
	let strategy = match (config_strategy, all_good) {
		(_, false) => PathDialog::InputBox,
		(s, true) => s,
	};
	let directory = dir::PROJECT_DIRECTORY.get();
	strategy.query(&directory, &codename).await
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Variable {
	TaskId,
	TaskTitle,
	ContestId,
	ContestTitle,
	SiteShort,
}

pub struct Mapping {
	pub task_id: Option<String>,
	pub task_title: Option<String>,
	pub contest_id: Option<String>,
	pub contest_title: Option<String>,
	pub site_short: Option<String>,
}

macro_rules! constrain_variable {
	($name:ident, $($matching:ident)|*) => {
		#[derive(Clone, Debug, PartialEq, Eq)]
		pub struct $name(Variable);
		impl crate::interpolation::VariableSet for $name {
			type Map = Mapping;

			fn expand(&self, map: &Self::Map) -> Option<String> {
				crate::interpolation::VariableSet::expand(&self.0, map)
			}
		}
		impl std::str::FromStr for $name {
			type Err = String;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				let v = $name(Variable::from_str(s)?);
				match v.0 {
					$(Variable::$matching => Ok($name(Variable::$matching)),)*
					#[allow(unreachable_patterns)]
					_ => Err(format!("variable {} not supported in {} context", v, stringify!($name))),
				}
			}
		}
		impl fmt::Display for $name {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				self.0.fmt(f)
			}
		}
	};
}

constrain_variable!(TaskVariable, TaskId | TaskTitle | ContestId | SiteShort);
constrain_variable!(ContestVariable, ContestId | ContestTitle | SiteShort);
constrain_variable!(ContestTaskVariable, TaskId | TaskTitle | ContestId | SiteShort);

impl crate::interpolation::VariableSet for Variable {
	type Map = Mapping;

	fn expand(&self, map: &Self::Map) -> Option<String> {
		match self {
			Variable::TaskId => map.task_id.clone(),
			Variable::TaskTitle => map.task_title.clone(),
			Variable::ContestId => map.contest_id.clone(),
			Variable::ContestTitle => map.contest_title.clone(),
			Variable::SiteShort => map.site_short.clone(),
		}
	}
}

impl FromStr for Variable {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"task.symbol" => Ok(Variable::TaskId),
			"task.name" => Ok(Variable::TaskTitle),
			"contest.id" => Ok(Variable::ContestId),
			"contest.title" => Ok(Variable::ContestTitle),
			"site.short" => Ok(Variable::SiteShort),
			_ => Err(format!("unrecognized variable name {:?}", s)),
		}
	}
}

impl fmt::Display for Variable {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			Variable::TaskId => "task.symbol",
			Variable::TaskTitle => "task.name",
			Variable::ContestId => "contest.id",
			Variable::ContestTitle => "contest.title",
			Variable::SiteShort => "site.short",
		})
	}
}
