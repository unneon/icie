use std::{error::Error as StdError, fmt, sync::PoisonError};
use wasm_backtrace::Backtrace;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorCode {
	/// Access was denied. This can be due to not being logged in, or not having access to the
	/// resource in general.
	AccessDenied,
	/// Some site decided to change the HTML structures, and scraping it failed. These errors need
	/// to be fixed as soon as they are reported, because users can't do anything about them.
	AlienInvasion,
	/// Some data (e.g. serialized session cookie) was deemed invalid. Repeating the operation
	/// without using the cache (e.g. logging in) should fix the error.
	MalformedData,
	/// An URL passed by the user or constructed internally was malformed.
	MalformedURL,
	/// A network request has failed for whatever reason. This can also be caused by infinite
	/// redirect loops and other reasons not strictly related to network status.
	NetworkFailure,
	/// The site denied submission or a different action, because the user was not registered for
	/// the contest. Some sites require accepting the terms and conditions before the contest, and
	/// if the user doesn't do this, this happens.
	NotRegistered,
	/// The resource requested was not yet made public. An expected publication time should be
	/// obtained through some other API.
	NotYetStarted,
	/// TLS configuration was not found. Never happens (?) on WASM, should never happen on any sane
	/// system.
	NoTLS,
	/// The server refused to handle an excessive amount of request, either by outright refusing
	/// the request or asking a captcha. ICIE's timeout are configured to avoid this, so it's
	/// likely the user's fault.
	RateLimit,
	/// Some internal concurrency issue has triggered a check. This likely happened due to some
	/// earlier panics.
	StateCorruption,
	/// User has passed a wrong username or password.
	WrongCredentials,
	/// User has passed an URL that was not recognized.
	WrongTaskUrl,
	// Contest has ended 
	Ended_Already
}

#[derive(Debug)]
pub struct Error {
	pub code: ErrorCode,
	pub cause: Option<Box<dyn StdError+'static>>,
	pub backtrace: Backtrace,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let message = match self.code {
			ErrorCode::AccessDenied => "access denied",
			ErrorCode::AlienInvasion => "website could not be analyzed",
			ErrorCode::MalformedData => "internal data corrupted possibly due to updates",
			ErrorCode::MalformedURL => "website link was not a valid URL",
			ErrorCode::NetworkFailure => "network request failed",
			ErrorCode::NotRegistered => "you are not registered for the contest",
			ErrorCode::NotYetStarted => "event has not yet started",
			ErrorCode::NoTLS => "could not find TLS configuration in your operating system",
			ErrorCode::RateLimit => "you sent too many requests to the website",
			ErrorCode::StateCorruption => "internal state corrupted due to earlier errors",
			ErrorCode::WrongCredentials => "wrong username or password",
			ErrorCode::WrongTaskUrl => "website link was not recognized",
			ErrorCode::Ended_Already => "Contest Ended Already",
		};
		write!(f, "{}", message)
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError+'static)> {
		self.cause.as_deref().map(|e| e as _)
	}
}

impl From<ErrorCode> for Error {
	fn from(code: ErrorCode) -> Self {
		Error { code, cause: None, backtrace: Backtrace::new() }
	}
}

impl From<debris::Error> for Error {
	fn from(e: debris::Error) -> Self {
		let backtrace = e.backtrace.clone();
		Error { code: ErrorCode::AlienInvasion, cause: Some(Box::new(e)), backtrace }
	}
}

impl<T> From<PoisonError<T>> for Error {
	fn from(_: PoisonError<T>) -> Self {
		Error {
			code: ErrorCode::StateCorruption,
			// While this isn't the original error, it will still provide a good error message (in
			// fact, the same message as the original in Rust 1.40).
			cause: Some(Box::new(PoisonError::new(()))),
			backtrace: Backtrace::new(),
		}
	}
}

impl From<reqwest::Error> for Error {
	fn from(e: reqwest::Error) -> Self {
		Error { code: ErrorCode::NetworkFailure, cause: Some(Box::new(e)), backtrace: Backtrace::new() }
	}
}

impl From<url::ParseError> for Error {
	fn from(e: url::ParseError) -> Self {
		Error { code: ErrorCode::MalformedURL, cause: Some(Box::new(e)), backtrace: Backtrace::new() }
	}
}
