use crate::util;
use evscode::R;

pub fn connect(url: &unijudge::TaskUrl) -> R<Box<dyn unijudge::Session>> {
	let (username, password) = {
		let _status = crate::STATUS.push("Remembering passwords");
		crate::auth::site_credentials(&url.site)?
	};
	let sess = {
		let _status = crate::STATUS.push("Logging in");
		unijudge::connect_login(&url.site, &username, &password).map_err(util::from_unijudge_error)?
	};
	Ok(sess)
}
