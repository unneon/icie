//! Rich error typee, supporting cancellation, backtraces, automatic logging and followup actions.
//!
//! It should also be used by extensions instead of custom error types, because it supports follow-up actions,
//! cancellations, hiding error details from the user, backtraces and carrying extended logs. Properly connecting these
//! features to VS Code API is a little bit code-heavy, and keeping this logic inside Evscode allows to improve error
//! message format across all extensions.

use futures::{
	stream::{once, select}, Stream, StreamExt
};
use std::{
	fmt, future::Future, ops::{ControlFlow, FromResidual, Try}, pin::Pin
};
use wasm_backtrace::Backtrace;

/// Result type used for errors in Evscode. See [`E`] for details.
pub type R<T> = Result<T, E>;

/// A button on an error message that the user can press.
pub struct Action {
	/// Title displayed to the user.
	/// Preferably one-word, because wider buttons look weird.
	pub title: String,
	/// The function that will be launched upon clicking the button.
	/// It will be called in a separate thread.
	pub trigger: Pin<Box<dyn Future<Output=R<()>>>>,
}

/// Indication of how serious the error is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Severity {
	/// Must 100% be a bug. Auto-report the error, show an error message including a link to GitHub
	/// issues, and possibly strongly urge the user to report this.
	Bug,
	/// May be an error caused by the user, or some far consequence of a bug. Auto-report the
	/// error, show an error message including a link to GitHub issues,
	Error,
	/// Like the [`Severity::Error`] variant, but when the error will not interrupt the requested
	/// operation. Auto-report the error, show a warning message including a link to GitHub issues.
	Warning,
	/// A typical error that occurs under normal plugin usage. Show an error message.
	Workflow,
	/// User requested to cancel an ongoing operation, for example but clicking a 'Cancel' button
	/// or closing an input window. Do nothing.
	Cancel,
}

/// Error type data used by Evscode.
#[derive(Debug)]
pub struct ErrorData {
	/// Marks whose fault this error is and how serious is it.
	pub severity: Severity,
	/// List of human-facing error messages, ordered from low-level to high-level.
	pub reasons: Vec<String>,
	/// List of actions available as buttons in the message, if displayed.
	pub actions: Vec<Action>,
	/// Stack trace from either where the error was converted to [`E`] or from a lower-level error.
	/// The backtrace will only be converted from foreign errors if it is done manually.
	pub backtrace: Backtrace,
	/// List of extended error logs, presumably too long to be displayed to the end user.
	pub extended: Vec<String>,
}

/// Error type used by Evscode. Boxed for code size.
///
/// See [module documentation](index.html) for details.
pub struct E(pub Box<ErrorData>);

impl E {
	/// Create an error from a user-facing string, capturing a backtrace.
	pub fn error(s: impl AsRef<str>) -> E {
		E::empty().context(s)
	}

	/// Create an error with no message.
	pub fn empty() -> E {
		E(Box::new(ErrorData {
			severity: Severity::Error,
			reasons: Vec::new(),
			actions: Vec::new(),
			backtrace: Backtrace::new(),
			extended: Vec::new(),
		}))
	}

	/// Create an error representing an operation cancelled by user. This error will be logged, but
	/// not displayed to the user.
	pub fn cancel() -> E {
		E::empty().severity(Severity::Cancel)
	}

	/// Convert an error implementing [`std::error::Error`] to an Evscode error. Error messages will
	/// be collected from [`std::fmt::Display`] implementations on each error in the
	/// [`std::error::Error::source`] chain.
	pub fn from_std(native: impl std::error::Error) -> E {
		E::from_std_ref(&native)
	}

	/// Convert an error reference implementing [`std::error::Error`] to an Evscode error. See
	/// [`E::from_std`] method for details.
	pub fn from_std_ref<E2: std::error::Error+?Sized>(native: &E2) -> E {
		let mut e = E::empty();
		e.0.reasons.push(format!("{}", native));
		let mut v: Option<&(dyn std::error::Error)> = native.source();
		while let Some(native) = v {
			let inner_message = format!("{}", native);
			if !e.0.reasons.iter().any(|reason| reason.contains(inner_message.as_str())) {
				e.0.reasons.push(inner_message);
			}
			v = native.source();
		}
		e.0.reasons.reverse();
		e
	}

	/// A short human-facing representation of the error.
	pub fn human(&self) -> String {
		let mut buf = String::new();
		for (i, reason) in self.0.reasons.iter().enumerate().rev() {
			buf += reason;
			if i != 0 {
				buf += "; ";
			}
		}
		buf
	}

	/// A human-facing representation of the error, but including internal error messages that are
	/// usually hidden.
	pub fn human_detailed(&self) -> String {
		let mut buf = String::new();
		for (i, reason) in self.0.reasons.iter().rev().enumerate() {
			buf += reason;
			if i != self.0.reasons.len() - 1 {
				buf += "; ";
			}
		}
		buf
	}

	/// Add an additional message describing the error, which will be displayed in front of the
	/// previous ones.
	///
	/// ```
	/// # use evscode::E;
	/// let e = E::error("DNS timed out")
	///     .context("network failure")
	///     .context("failed to fetch Bitcoin prices");
	/// assert_eq!(
	///     e.human(),
	///     "failed to fetch Bitcoin prices; network failure; DNS timed out"
	///     );
	/// ```
	pub fn context(mut self, msg: impl AsRef<str>) -> Self {
		self.0.reasons.push(msg.as_ref().to_owned());
		self
	}

	/// Add a follow-up action that can be taken by the user, who will see the action as a button on
	/// the error message.
	pub fn action(mut self, title: impl AsRef<str>, trigger: impl Future<Output=R<()>>+'static) -> Self {
		self.0.actions.push(Action { title: title.as_ref().to_owned(), trigger: Box::pin(trigger) });
		self
	}

	/// A convenience function to add a follow-up action if the condition is true. See [`E::action`]
	/// for details.
	pub fn action_if(self, cond: bool, title: impl AsRef<str>, trigger: impl Future<Output=R<()>>+'static) -> Self {
		if cond { self.action(title, trigger) } else { self }
	}

	/// Add an extended error log, which typically is a multiline string, like a compilation log or
	/// a subprocess output. The log will be displayed as a seperate message in developer tools.
	pub fn extended(mut self, extended: impl AsRef<str>) -> Self {
		self.0.extended.push(extended.as_ref().to_owned());
		self
	}

	/// Set the severity of an error. This will affect how the error is displayed, and whether it
	/// will be auto-reported.
	pub fn severity(mut self, severity: Severity) -> Self {
		self.0.severity = severity;
		self
	}

	/// Checks whether this error should be automatically sent to error collection systems. See
	/// [`Severity`] for details.
	pub fn should_auto_report(&self) -> bool {
		match self.0.severity {
			Severity::Bug => true,
			Severity::Error => true,
			Severity::Warning => true,
			Severity::Workflow => false,
			Severity::Cancel => false,
		}
	}

	/// Prints the error to logs and displays a message to the user, if necessary. Prefer returning error results from
	/// event handlers rather than calling this function directly.
	pub fn emit(self) {
		self.emit_log();
		self.emit_user();
	}

	fn should_show(&self) -> bool {
		match self.0.severity {
			Severity::Bug => true,
			Severity::Error => true,
			Severity::Warning => true,
			Severity::Workflow => true,
			Severity::Cancel => false,
		}
	}

	/// Prints the error to logging systems.
	pub fn emit_log(&self) {
		if self.should_show() {
			let mut log_msg = String::new();
			for reason in &self.0.reasons {
				log_msg += &format!("{}\n", reason);
			}
			log_msg += &format!("\n{:?}", self.0.backtrace);
			log::error!("{}", log_msg);
			for extended in &self.0.extended {
				log::error!("{}", extended);
			}
		}
	}

	/// Displays the error message to the user.
	pub fn emit_user(self) {
		if self.should_show() {
			let should_suggest_report = match self.0.severity {
				Severity::Bug => true,
				Severity::Error => true,
				Severity::Warning => true,
				Severity::Workflow => false,
				Severity::Cancel => false,
			};
			let message = format!(
				"{}{}",
				self.human(),
				if should_suggest_report { "; [report issue?](https://github.com/pustaczek/icie/issues)" } else { "" }
			);
			let items = self.0.actions.iter().enumerate().map(|(id, action)| crate::message::Action {
				id,
				title: &action.title,
				is_close_affordance: false,
			});
			let msg = crate::Message::new(&message).items(items);
			let msg = match self.0.severity {
				Severity::Bug => msg.error(),
				Severity::Error => msg.error(),
				Severity::Warning => msg.warning(),
				Severity::Workflow => msg.error(),
				Severity::Cancel => msg,
			};
			let promise = msg.show_eager();
			crate::spawn(async move {
				let choice = promise.await;
				if let Some(choice) = choice {
					let action = self.0.actions.into_iter().nth(choice).unwrap();
					action.trigger.await?;
				}
				Ok(())
			});
		}
	}
}

impl fmt::Debug for E {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self.0, f)
	}
}

/// An extension trait for terser error handling.
pub trait ResultExt {
	/// The value type of the result.
	type Ok;
	/// Convert the error to [`E`] and add a single context layer.
	fn wrap(self, s: impl AsRef<str>) -> R<Self::Ok>;
}
impl<T, E2: std::error::Error> ResultExt for Result<T, E2> {
	type Ok = T;

	fn wrap(self, s: impl AsRef<str>) -> R<T> {
		self.map_err(|e| E::from_std(e).context(s))
	}
}
impl<T> ResultExt for Option<T> {
	type Ok = T;

	fn wrap(self, s: impl AsRef<str>) -> R<T> {
		self.ok_or_else(|| E::error(s))
	}
}

/// Error type representing a operation intentionally cancelled by the user.
pub struct Cancellation;

/// Result-like type for operations that could be intentionally cancelled by the user.
///
/// It implements [`std::ops::Try`], which makes it possible to use ? operator in functions returning [`R`].
#[derive(Debug)]
pub struct Cancellable<T>(pub Option<T>);

/// Return a stream yielding values from this stream, unless the other future yields any value. In that case the stream
/// will yield a Result-like value representing a cancelled operation, that can be forwarder using the ? operator.
pub fn cancel_on<T, A: Stream<Item=T>, B: Future<Output=()>>(a: A, b: B) -> impl Stream<Item=Cancellable<T>> {
	select(a.map(|x| Cancellable(Some(x))), once(b).map(|()| Cancellable(None)))
}

impl<T> Try for Cancellable<T> {
	type Output = T;
	type Residual = Cancellation;

	fn from_output(v: Self::Output) -> Self {
		Cancellable(Some(v))
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		match self.0 {
			Some(v) => ControlFlow::Continue(v),
			None => ControlFlow::Break(Cancellation),
		}
	}
}

impl<T> FromResidual<Cancellation> for Cancellable<T> {
	fn from_residual(_: Cancellation) -> Self {
		Cancellable(None)
	}
}

impl FromResidual<Cancellation> for Result<(), E> {
	fn from_residual(_: Cancellation) -> Self {
		Err(E::cancel())
	}
}

impl From<js_sys::Error> for E {
	fn from(e: js_sys::Error) -> Self {
		E(Box::new(ErrorData {
			severity: Severity::Error,
			reasons: vec![String::from(e.message())],
			actions: Vec::new(),
			backtrace: Backtrace(e),
			extended: Vec::new(),
		}))
	}
}

impl fmt::Debug for Action {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Action").field("title", &self.title).finish()
	}
}
