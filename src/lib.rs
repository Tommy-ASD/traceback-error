pub mod block_on;
pub mod set_callback;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use set_callback::TracebackCallbackType;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs::File,
    io::Write,
};

pub use serde_json;

pub static mut TRACEBACK_ERROR_CALLBACK: Option<TracebackCallbackType> = None;

/// A custom error struct for handling tracebacks in Rust applications.
///
/// This struct is designed to capture error information such as the error message,
/// the file and line where the error occurred, and additional contextual data.
///
/// # Examples
///
/// Creating a new `TracebackError` with a custom message:
///
/// ```rust
/// use chrono::{DateTime, Utc};
/// use serde_json::Value;
/// use traceback_error::TracebackError;
///
/// let error = traceback!("Custom error message");
/// println!("{:?}", error);
/// ```
///
/// # Fields
///
/// - `message`: A string containing the error message.
/// - `file`: A string containing the filename where the error occurred.
/// - `line`: An unsigned integer representing the line number where the error occurred.
/// - `parent`: An optional boxed `TracebackError` representing the parent error, if any.
/// - `time_created`: A `chrono::DateTime<Utc>` indicating when the error was created.
/// - `extra_data`: A `serde_json::Value` for storing additional error-related data.
/// - `project`: An optional string representing the project name.
/// - `computer`: An optional string representing the computer name.
/// - `user`: An optional string representing the username.
/// - `is_parent`: A boolean indicating if this error is considered a parent error.
/// - `is_handled`: A boolean indicating if the error has been handled.
/// - `is_default`: A boolean indicating if this error is the default error.
///
/// # Default Implementation
///
/// The `Default` trait is implemented for `TracebackError`, creating a default instance
/// with the following values:
///
/// - `message`: "Default message"
/// - `file`: The current file's name (using `file!()`).
/// - `line`: The current line number (using `line!()`).
/// - `parent`: None
/// - `time_created`: The Unix epoch time.
/// - `extra_data`: Value::Null
/// - `project`: None
/// - `computer`: None
/// - `user`: None
/// - `is_parent`: false
/// - `is_handled`: false
/// - `is_default`: true
///
/// # Equality Comparison
///
/// The `PartialEq` trait is implemented for `TracebackError`, allowing you to compare
/// two `TracebackError` instances for equality based on their message, file, line, and
/// other relevant fields. The `is_handled` and `is_default` fields are not considered
/// when comparing for equality.
///
/// # Dropping Errors
///
/// Errors are automatically dropped when they go out of scope, but before they are dropped,
/// they are handled by the `TRACEBACK_ERROR_CALLBACK` function.
/// By default, this function simply serializes the error and writes it to a JSON file.
///
/// # Callback Types
///
/// The callback function can be either synchronous or asynchronous, depending on the
/// `TracebackCallbackType` set globally using the `TRACEBACK_ERROR_CALLBACK` variable.
/// It can be set using the `set_callback!` macro.
///
/// - If `TRACEBACK_ERROR_CALLBACK` is `Some(TracebackCallbackType::Async)`, an
///   asynchronous callback function is used.
/// - If `TRACEBACK_ERROR_CALLBACK` is `Some(TracebackCallbackType::Sync)`, a
///   synchronous callback function is used.
/// - If `TRACEBACK_ERROR_CALLBACK` is `None`, a default callback function is used.
///
/// # Creating Errors
///
/// You can create a new `TracebackError` instance using the `traceback!` macro. Additional
/// data can be added using the `with_extra_data` method, and environment variables are
/// automatically added when the error is being handled.
///
/// # Environment Variables
///
/// The `with_env_vars` method populates the `project`, `computer`, and `user` fields with
/// information obtained from environment variables (`CARGO_PKG_NAME`, `COMPUTERNAME`, and
/// `USERNAME`, respectively) or assigns default values if the environment variables are
/// not present.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TracebackError {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub parent: Option<Box<TracebackError>>,
    pub time_created: DateTime<Utc>,
    pub extra_data: Value,
    pub project: Option<String>,
    pub computer: Option<String>,
    pub user: Option<String>,
    pub is_parent: bool,
    pub is_handled: bool,
    is_default: bool,
}

impl Default for TracebackError {
    fn default() -> Self {
        Self {
            message: "Default message".to_string(),
            file: file!().to_string(),
            line: line!(),
            parent: None,
            time_created: DateTime::from_utc(
                chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
                Utc,
            ),
            extra_data: Value::Null,
            project: None,
            computer: None,
            user: None,
            is_parent: false,
            is_handled: false,
            is_default: true,
        }
    }
}

impl PartialEq for TracebackError {
    fn eq(&self, other: &Self) -> bool {
        let (this, mut other) = (self.clone(), other.clone());
        other.is_handled = this.is_handled;
        this.message == other.message
            && this.file == other.file
            && this.line == other.line
            && this.parent == other.parent
            && this.extra_data == other.extra_data
            && this.project == other.project
            && this.computer == other.computer
            && this.user == other.user
            && this.is_parent == other.is_parent
    }
}

impl Drop for TracebackError {
    fn drop(&mut self) {
        if self.is_parent || self.is_handled || self.is_default {
            return;
        }
        let mut this = std::mem::take(self);
        this.is_handled = true;
        unsafe {
            let callback: Option<&mut TracebackCallbackType> = TRACEBACK_ERROR_CALLBACK.as_mut();
            match callback {
                Some(TracebackCallbackType::Async(ref mut f)) => {
                    block_on::block_on(f.call(this)); // bad practice, fix later
                }
                Some(TracebackCallbackType::Sync(ref mut f)) => {
                    f.call(this);
                }
                None => {
                    default_callback(this);
                }
            }
        }
    }
}

impl TracebackError {
    pub fn new(message: String, file: String, line: u32) -> Self {
        Self {
            message,
            file,
            line,
            parent: None,
            time_created: Utc::now(),
            extra_data: Value::Null,
            project: None,
            computer: None,
            user: None,
            is_parent: false,
            is_handled: false,
            is_default: false,
        }
    }
    pub fn with_extra_data(mut self, extra_data: Value) -> Self {
        self.is_default = false;
        self.extra_data = extra_data;
        self
    }
    pub fn with_env_vars(mut self) -> Self {
        // get project name using the CARGO_PKG_NAME env variable
        let project_name = match std::env::var("CARGO_PKG_NAME") {
            Ok(p) => p,
            Err(_) => "Unknown due to CARGO_PKG_NAME missing".to_string(),
        };
        // get computer name using the COMPUTERNAME env variable
        let computer_name = match std::env::var("COMPUTERNAME") {
            Ok(c) => c,
            Err(_) => "Unknown due to COMPUTERNAME missing".to_string(),
        };
        // get username using the USERNAME env variable
        let username = match std::env::var("USERNAME") {
            Ok(u) => u,
            Err(_) => "Unknown due to USERNAME missing".to_string(),
        };
        self.is_default = false;
        self.project = Some(project_name);
        self.computer = Some(computer_name);
        self.user = Some(username);
        self
    }
    pub fn with_parent(mut self, parent: TracebackError) -> Self {
        self.is_default = false;
        self.parent = Some(Box::new(parent.with_is_parent(true)));
        self
    }
    fn with_is_parent(mut self, is_parent: bool) -> Self {
        self.is_default = false;
        self.is_parent = is_parent;
        self
    }
}

/// This display implementation is recursive, and will print the error and all its parents
/// with a tab in front of each parent.
impl Display for TracebackError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut parent = self.parent.as_ref();
        let mut first = true;
        let mut amount_tabs = 0;
        while let Some(p) = parent {
            if first {
                first = false;
            } else {
                write!(f, "\n")?;
            }
            for _ in 0..amount_tabs {
                write!(f, "\t")?;
            }
            write!(f, "{}", p)?;
            amount_tabs += 1;
            parent = p.parent.as_ref();
        }
        write!(f, "\n")?;
        for _ in 0..amount_tabs {
            write!(f, "\t")?;
        }
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

impl Error for TracebackError {}

impl serde::de::Error for TracebackError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        // Create a new TracebackError with the provided message
        TracebackError {
            message: msg.to_string(),
            file: String::new(),
            line: 0,
            parent: None,
            time_created: Utc::now(),
            extra_data: json!({
                "error_type": "serde::de::Error",
                "error_message": msg.to_string()
            }),
            project: None,
            computer: None,
            user: None,
            is_parent: false,
            is_handled: false,
            is_default: false,
        }
    }
}

pub fn default_callback(err: TracebackError) {
    let err = err.with_env_vars();

    // get current time
    let current_time = chrono::Utc::now();
    let current_time_string = current_time.format("%Y-%m-%d.%H-%M-%S").to_string();
    let nanosecs = current_time.timestamp_nanos();
    let current_time_string = format!("{}.{}", current_time_string, nanosecs);
    // check if errors folder exists
    match std::fs::read_dir("errors") {
        Ok(_) => {}
        Err(_) => {
            // if not, create it
            match std::fs::create_dir("errors") {
                Ok(_) => {}
                Err(e) => {
                    println!("Error when creating directory: {}", e);
                    return;
                }
            };
        }
    };
    // create {current_time_string}.json
    let filename = format!("./errors/{current_time_string}.json");
    println!("Writing error to file: {}", filename);
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(e) => {
            println!("Error when creating file: {}", e);
            return;
        }
    };
    // parse error to json
    let err = match serde_json::to_string_pretty(&err) {
        Ok(e) => e,
        Err(e) => {
            println!("Error when parsing error: {}", e);
            return;
        }
    };
    // write json to file
    match file.write_all(err.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            println!("Error when writing to file: {}", e);
            return;
        }
    };
}
/// A macro for creating instances of the `TracebackError` struct with various options.
///
/// The `traceback!` macro simplifies the creation of `TracebackError` instances by providing
/// convenient syntax for specifying error messages and handling different error types.
///
/// # Examples
///
/// Creating a new `TracebackError` with a custom message:
///
/// ```rust
/// use traceback_error::traceback;
///
/// let error = traceback!("Custom error message");
/// println!("{:?}", error);
/// ```
///
/// Creating a new `TracebackError` from a generic error:
///
/// ```rust
/// use traceback_error::traceback;
///
/// fn custom_function() -> Result<(), Box<dyn std::error::Error>> {
///     // ...
///     // Some error occurred
///     let generic_error: Box<dyn std::error::Error> = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Generic error"));
///     Err(traceback!(err generic_error))
/// }
/// ```
///
/// Creating a new `TracebackError` from a generic error with a custom message:
///
/// ```rust
/// use traceback_error::traceback;
///
/// fn custom_function() -> Result<(), Box<dyn std::error::Error>> {
///     // ...
///     // Some error occurred
///     let generic_error: Box<dyn std::error::Error> = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Generic error"));
///     Err(traceback!(generic_error, "Custom error message"))
/// }
/// ```
///
/// # Syntax
///
/// The `traceback!` macro supports the following syntax variations:
///
/// - `traceback!()`: Creates a `TracebackError` with an empty message, using the current file
///   and line number.
///
/// - `traceback!($msg:expr)`: Creates a `TracebackError` with the specified error message,
///   using the current file and line number.
///
/// - `traceback!(err $e:expr)`: Attempts to downcast the provided error (`$e`) to a
///   `TracebackError`. If successful, it marks the error as handled and creates a new
///   `TracebackError` instance based on the downcasted error. If the downcast fails, it
///   creates a `TracebackError` with an empty message and includes the original error's
///   description in the extra data field.
///
/// - `traceback!($e:expr, $msg:expr)`: Similar to the previous variation but allows specifying
///   a custom error message for the new `TracebackError` instance.
///
/// # Error Handling
///
/// When using the `traceback!` macro to create `TracebackError` instances from other error types,
/// it automatically sets the `is_handled` flag to `true` for the original error to indicate that
/// it has been handled. This prevents the `TRACEBACK_ERROR_CALLBACK` function to be called on it.
///
/// # Environment Variables
///
/// Environment variables such as `CARGO_PKG_NAME`, `COMPUTERNAME`, and `USERNAME` are automatically
/// added to the `project`, `computer`, and `user` fields when the error is being handled.
///
#[macro_export]
macro_rules! traceback {
    () => {
        $crate::TracebackError::new("".to_string(), file!().to_string(), line!())
    };
    ($msg:expr) => {
        $crate::TracebackError::new($msg.to_string(), file!().to_string(), line!())
    };
    (err $e:expr) => {{
        use $crate::serde_json::json;
        let err_string = $e.to_string();
        let mut boxed: Box<dyn std::any::Any> = Box::new($e);
        if let Some(traceback_err) = boxed.downcast_mut::<$crate::TracebackError>() {
            traceback_err.is_handled = true;
            $crate::TracebackError::new(
                traceback_err.message.to_string(),
                file!().to_string(),
                line!(),
            )
            .with_parent(traceback_err.clone())
        } else {
            $crate::TracebackError::new(String::from(""), file!().to_string(), line!())
                .with_extra_data(json!({
                    "error": err_string
                }))
        }
    }};
    ($e:expr, $msg:expr) => {{
        use $crate::serde_json::json;
        let err_string = $e.to_string();
        let mut boxed: Box<dyn std::any::Any> = Box::new($e);
        if let Some(traceback_err) = boxed.downcast_mut::<$crate::TracebackError>() {
            traceback_err.is_handled = true;
            $crate::TracebackError::new(
                $msg.to_string(),
                file!().to_string(),
                line!(),
            )
            .with_parent(traceback_err.clone())
        } else {
            $crate::TracebackError::new(String::from(""), file!().to_string(), line!())
                .with_extra_data(json!({
                    "error": err_string
                }))
        }
    }};
}
