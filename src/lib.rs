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

pub use paste;
pub use serde_json;

/// # Traceback Error Callback
///
/// The `TRACEBACK_ERROR_CALLBACK` is a mutable static variable that holds an
/// optional callback function for custom error handling in a Rust program using
/// the `traceback_error` crate. This callback is called when a `TracebackError`
/// goes out of scope, allowing you to customize how error information is handled
/// and reported.
///
/// ## Usage
///
/// To use the `TRACEBACK_ERROR_CALLBACK`, you can set it to your custom traceback
/// callback function using the `set_traceback!` macro.
/// Your custom traceback callback function should take an argument of type
/// `traceback_error::TracebackError`. The macro generates a unique struct and
/// function to wrap your callback and sets it as the traceback callback.
///
/// Example of setting a custom traceback callback:
///
/// ```rust
/// // Define a custom traceback callback function
/// fn my_traceback_callback(error: traceback_error::TracebackError) {
///     // Custom error handling logic here
///     println!("Custom traceback callback called: {:?}", error);
/// }
///
/// // Use the set_traceback macro to set the custom traceback callback
/// traceback_error::set_traceback!(my_traceback_callback);
///
/// // Any TracebackErrors will now be handled by my_traceback_callback when dropped
/// ```
///
/// ## Asynchronous Callbacks
///
/// If your custom traceback callback is asynchronous, you can specify it as such
/// using the `async` keyword when calling the `set_traceback!` macro.
///
/// Example of setting an asynchronous custom traceback callback:
///
/// ```rust
/// // Define an asynchronous custom traceback callback function
/// async fn my_async_traceback_callback(error: traceback_error::TracebackError) {
///     // Custom error handling logic here
///     println!("Async custom traceback callback called: {:?}", error);
/// }
///
/// // Use the set_traceback macro to set the asynchronous custom traceback callback
/// traceback_error::set_traceback!(async my_async_traceback_callback);
/// ```
pub static mut TRACEBACK_ERROR_CALLBACK: Option<TracebackCallbackType> = None;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ErrorLevel {
    None,
    Unknown,
    Log,
    Debug,
    Warn,
    Error,
    Other(String),
}

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
///
/// let error = traceback_error::traceback!("Custom error message");
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
/// they are handled by the `TRACEBACK_ERROR_CALLBACK` variable.
/// By default, this variable is a function simply set to serialize the error and
/// write it to a JSON file, but the default function can be changed with the
/// `set_callback!` macro.
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
/// The additional data should be stored in a serde_json::Value struct.
///
/// # Environment Variables
///
/// The `with_env_vars` method populates the `project`, `computer`, and `user` fields with
/// information obtained from environment variables (`CARGO_PKG_NAME`, `COMPUTERNAME`, and
/// `USERNAME`, respectively) or assigns default values if the environment variables are
/// not present.
///
/// # Tracing
///
/// Tracing can be essential for diagnosing and debugging issues in your applications. When an
/// error occurs, you can create a `TracebackError` instance to record the error's details, such
/// as the error message, the location in the code where it occurred, and additional contextual
/// information.
/// Should a function return a TracebackError, it can then be re-captured to trace it even further.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TracebackError {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub parent: Option<Box<TracebackError>>,
    pub time_created: DateTime<Utc>,
    pub extra_data: Vec<Value>,
    pub project: Option<String>,
    pub computer: Option<String>,
    pub user: Option<String>,
    pub is_parent: bool,
    pub is_handled: bool,
    pub level: ErrorLevel,
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
            extra_data: vec![],
            project: None,
            computer: None,
            user: None,
            is_parent: false,
            is_handled: false,
            is_default: true,
            level: ErrorLevel::Log,
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
        this = this.with_env_vars();
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
    pub fn new(message: String, file: String, line: u32, level: ErrorLevel) -> Self {
        Self {
            message,
            file,
            line,
            parent: None,
            time_created: Utc::now(),
            extra_data: vec![],
            project: None,
            computer: None,
            user: None,
            is_parent: false,
            is_handled: false,
            is_default: false,
            level,
        }
    }
    /// This method allows you to attach additional data to a `TracebackError` instance.
    /// This extra data can be valuable when diagnosing and debugging errors,
    /// as it provides context and information related to the error.
    ///
    /// ## Parameters:
    /// - `extra_data`: A `serde_json::Value` containing the extra data you want to associate with the error.
    ///
    /// ## Return Value:
    /// - Returns a modified `TracebackError` instance with the provided `extra_data`.
    ///
    /// ## Example Usage:
    /// ```rs
    /// use traceback_error::{traceback, TracebackError, serde_json::json};
    ///
    /// fn main() {
    ///     // Create a new TracebackError with extra data
    ///     let error = traceback!().with_extra_data(json!({
    ///         "foo": "bar",
    ///         "a": "b",
    ///         "1": "2"
    ///     }));
    ///
    ///     // Now the error instance contains the specified extra data
    /// }
    /// ```
    ///
    /// This method is useful when you want to enrich error objects with additional information
    /// relevant to the context in which the error occurred. It ensures that relevant data is
    /// available for analysis when handling errors in your Rust application.
    pub fn with_extra_data(mut self, extra_data: Value) -> Self {
        self.is_default = false;
        self.extra_data.push(extra_data);
        self
    }
    /// Adds environment variables to the TracebackError.
    ///
    /// This method populates the `project`, `computer`, and `user` fields of the `TracebackError`
    /// based on the values of specific environment variables. If any of these environment variables
    /// are not found, default values are used, and the error message reflects that the information
    /// is unknown due to the missing environment variables.
    ///
    /// # Example:
    ///
    /// ```
    /// use traceback_error::TracebackError;
    ///
    /// // Create a new TracebackError and populate environment variables
    /// let error = TracebackError::new("An error occurred".to_string(), file!().to_string(), line!())
    ///     .with_env_vars();
    ///
    /// // The error now contains information about the project, computer, and user from
    /// // environment variables, or default values if the environment variables are missing.
    /// ```
    ///
    /// # Environment Variables Used:
    ///
    /// - `CARGO_PKG_NAME`: Used to set the `project` field.
    /// - `COMPUTERNAME`: Used to set the `computer` field.
    /// - `USERNAME`: Used to set the `user` field.
    ///
    /// # Returns:
    ///
    /// A modified `TracebackError` with updated `project`, `computer`, and `user` fields.
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
    /// The `with_parent` method allows you to associate a parent error with the current `TracebackError` instance.
    /// This can be useful when you want to create a hierarchical structure of errors, where one error is considered the parent of another.
    ///
    /// ## Parameters:
    /// - `parent`: A `TracebackError` instance that you want to set as the parent of the current error.
    ///
    /// ## Return Value:
    /// - Returns a modified `TracebackError` instance with the specified parent error.
    ///
    /// ## Example:
    /// ```rs
    /// use traceback_error::TracebackError;
    ///
    /// fn main() {
    ///     // Create a new TracebackError
    ///     let parent_error = TracebackError::new("Parent error".to_string(), file!().to_string(), line!());
    ///
    ///     // Create a child error with the parent error
    ///     let child_error = TracebackError::new("Child error".to_string(), file!().to_string(), line!())
    ///         .with_parent(parent_error);
    ///
    ///     // Now, `child_error` has `parent_error` as its parent
    /// }
    /// ```
    ///
    /// The with_parent method is particularly useful when you want to establish relationships between errors,
    /// making it easier to understand error hierarchies and diagnose issues.
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
            extra_data: vec![json!({
                "error_type": "serde::de::Error",
                "error_message": msg.to_string()
            })],
            project: None,
            computer: None,
            user: None,
            is_parent: false,
            is_handled: false,
            is_default: false,
            level: ErrorLevel::Log,
        }
    }
}

/// # Default Traceback Error Callback
///
/// The `default_callback` function is a built-in error handling callback used
/// when `TRACEBACK_ERROR_CALLBACK` is set to `None`. This callback is responsible
/// for handling and reporting `TracebackError` instances in a default manner.
///
/// ## Behavior
///
/// When a `TracebackError` goes out of scope and `TRACEBACK_ERROR_CALLBACK` is not
/// set to a custom callback, the `default_callback` function is used. This function
/// performs the following actions:
///
/// 1. Retrieves the current time in UTC and creates a timestamp string.
/// 2. Checks if the "errors" folder exists and creates it if it doesn't.
/// 3. Generates a unique filename based on the current timestamp.
/// 4. Writes the error information in JSON format to a file with the generated filename
///    in the "errors" folder.
/// 5. Logs any encountered errors during the above steps.
///
/// This default behavior ensures that unhandled errors are captured, timestamped,
/// and saved as JSON files for later analysis.
///
/// ## Usage
///
/// Typically, you don't need to call the `default_callback` function directly. Instead,
/// it is automatically used as the error handler when `TRACEBACK_ERROR_CALLBACK` is not
/// set to a custom callback.
///
/// Example of using the default behavior when `TRACEBACK_ERROR_CALLBACK` is not set:
///
/// ```rust
/// // No custom callback set, so the default_callback will be used
/// traceback_error::set_traceback!(None);
///
/// // Any TracebackErrors will now be handled by the default_callback when dropped
/// ```
///
/// To customize error handling, you can set a custom callback using the `set_traceback!`
/// macro as shown in the documentation for `TRACEBACK_ERROR_CALLBACK`.
pub fn default_callback(err: TracebackError) {
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
/// let error = traceback_error::traceback!("Custom error message");
/// println!("{:?}", error);
/// ```
///
/// Creating a new `TracebackError` from a generic error:
///
/// ```rust
/// fn custom_function() -> Result<(), traceback_error::TracebackError> {
///     // ...
///     // Some error occurred
///     let generic_error: Box<dyn std::error::Error> = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Generic error"));
///     Err(traceback_error::traceback!(err generic_error))
/// }
/// ```
///
/// Creating a new `TracebackError` from a generic error with a custom message:
///
/// ```rust
/// fn custom_function() -> Result<(), traceback_error::TracebackError> {
///     // ...
///     // Some error occurred
///     let generic_error: Box<dyn std::error::Error> = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Generic error"));
///     Err(traceback_error::traceback!(err generic_error, "Custom error message"))
/// }
/// ```
///
/// Tracing an error:
/// ```rust
/// fn main() {
///     match caller_of_tasks() {
///         Ok(_) => {}
///         Err(e) => {
///             traceback_error::traceback!(err e, "One of the tasks failed");
///         }
///     }
/// }
///
/// fn task_that_may_fail() -> Result<(), traceback_error::TracebackError> {
///     return Err(traceback_error::traceback!("task_that_may_fail failed"));
/// }
///
/// fn other_task_that_may_fail() -> Result<(), traceback_error::TracebackError> {
///     return Err(traceback_error::traceback!("other_task_that_may_fail failed"));
/// }
///
/// fn caller_of_tasks() -> Result<(), traceback_error::TracebackError> {
///     match task_that_may_fail() {
///         Ok(_) => {}
///         Err(e) => {
///             return Err(traceback_error::traceback!(err e));
///         }
///     };
///     match other_task_that_may_fail() {
///         Ok(_) => {}
///         Err(e) => {
///             return Err(traceback_error::traceback!(err e));
///         }
///     };
///     Ok(())
/// }
/// ```
/// When the error is dropped at the end of main() in the above example, the default callback
/// function generates the following JSON error file:
/// ```json
/// {
///   "message": "One of the tasks failed",
///   "file": "src\\main.rs",
///   "line": 7,
///   "parent": {
///     "message": "task_that_may_fail failed",
///     "file": "src\\main.rs",
///     "line": 24,
///     "parent": {
///       "message": "task_that_may_fail failed",
///       "file": "src\\main.rs",
///       "line": 13,
///       "parent": null,
///       "time_created": "2023-09-11T10:27:25.195697400Z",
///       "extra_data": null,
///       "project": null,
///       "computer": null,
///       "user": null,
///       "is_parent": true,
///       "is_handled": true,
///       "is_default": false
///     },
///     "time_created": "2023-09-11T10:27:25.195789100Z",
///     "extra_data": null,
///     "project": null,
///     "computer": null,
///     "user": null,
///     "is_parent": true,
///     "is_handled": true,
///     "is_default": false
///   },
///   "time_created": "2023-09-11T10:27:25.195836Z",
///   "extra_data": null,
///   "project": "traceback_test",
///   "computer": "tommypc",
///   "user": "tommy",
///   "is_parent": false,
///   "is_handled": true,
///   "is_default": false
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
/// - `traceback!(err $e:expr, $msg:expr)`: Similar to the previous variation but allows specifying
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
// TODO: make macro support error levels
#[macro_export]
macro_rules! traceback {
    () => {
        $crate::TracebackError::new("".to_string(), file!().to_string(), line!(), $crate::ErrorLevel::Unknown)
    };
    ($msg:expr) => {
        $crate::TracebackError::new($msg.to_string(), file!().to_string(), line!(), $crate::ErrorLevel::Unknown)
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
                $crate::ErrorLevel::Unknown,
            )
            .with_parent(traceback_err.clone())
        } else {
            $crate::TracebackError::new(String::from(""), file!().to_string(), line!(), $crate::ErrorLevel::Unknown)
                .with_extra_data(json!({
                    "error": err_string
                }))
        }
    }};
    (err $e:expr, $msg:expr) => {{
        use $crate::serde_json::json;
        let err_string = $e.to_string();
        let mut boxed: Box<dyn std::any::Any> = Box::new($e);
        if let Some(traceback_err) = boxed.downcast_mut::<$crate::TracebackError>() {
            traceback_err.is_handled = true;
            $crate::TracebackError::new(
                $msg.to_string(),
                file!().to_string(),
                line!(),
                $crate::ErrorLevel::Unknown,
            )
            .with_parent(traceback_err.clone())
        } else {
            $crate::TracebackError::new($msg.to_string(), file!().to_string(), line!(), $crate::ErrorLevel::Unknown)
                .with_extra_data(json!({
                    "error": err_string
                }))
        }
    }};
}
