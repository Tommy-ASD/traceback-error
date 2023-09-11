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

pub static mut TRACEBACK_ERROR_CALLBACK: Option<TracebackCallbackType> = None;

// This struct is getting messier by the minute
// To whoever's job it becomes refactoring this:
// Please accept my apologies, and good luck
// I could maybe attempt to explain what i was thinking when i made this,
// but i don't think i could do it justice
// I am sorry
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
        // && self.is_handled == other.is_handled
        // this should not be compared, as it is not a part of the error
    }
}

impl Drop for TracebackError {
    // for anyone (including me) reading this in the future
    // i am sorry for this mess
    // this was made at a time i was new to memory management
    // TODO: come back when more knowledgeable
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

/// Cloning this may be expensive, but for now it's fine
/// The reason it may be expensive is because it recursively clones the parent
/// and the parent's parent and so on
/// (i think)
/// To fix this, we could make it so that each of the with_ functions consume self,
/// and then return a new Self with the new data
/// Very easy fix, but i am unsure if we'll need the possibility to keep the old error
/// Maybe make non-consuming and consuming versions?
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
    pub fn with_parent(mut self, parent: TracebackError) -> Self {
        self.is_default = false;
        self.parent = Some(Box::new(parent.with_is_parent(true)));
        self
    }
    pub fn with_extra_data(mut self, extra_data: Value) -> Self {
        self.is_default = false;
        self.extra_data = extra_data;
        self
    }
    pub fn with_project(mut self, project: &str) -> Self {
        self.is_default = false;
        self.project = Some(project.to_string());
        self
    }
    pub fn with_computer_name(mut self, computer: &str) -> Self {
        self.is_default = false;
        self.computer = Some(computer.to_string());
        self
    }
    pub fn with_username(mut self, user: &str) -> Self {
        self.is_default = false;
        self.user = Some(user.to_string());
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
    pub fn with_is_parent(mut self, is_parent: bool) -> Self {
        self.is_default = false;
        self.is_parent = is_parent;
        self
    }
    pub fn clone(&mut self) -> Self {
        let handled = self.is_handled;
        self.is_handled = true;
        Self {
            message: self.message.clone(),
            line: self.line.clone(),
            file: self.file.clone(),
            parent: self.parent.clone(),
            time_created: self.time_created.clone(),
            extra_data: self.extra_data.clone(),
            project: self.project.clone(),
            computer: self.computer.clone(),
            user: self.user.clone(),
            is_parent: self.is_parent.clone(),
            is_handled: handled,
            is_default: self.is_default.clone(),
        }
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

#[macro_export]
macro_rules! traceback {
    () => {
        $crate::error_types::TracebackError::new("".to_string(), file!().to_string(), line!())
    };
    ($msg:expr) => {
        $crate::error_types::TracebackError::new($msg.to_string(), file!().to_string(), line!())
    };
    (err $e:expr) => {{
        use $crate::serde_json::json;
        let err_string = $e.to_string();
        let mut boxed: Box<dyn std::any::Any> = Box::new($e);
        if let Some(traceback_err) = boxed.downcast_mut::<TracebackError>() {
            traceback_err.is_handled = true;
            $crate::error_types::TracebackError::new(
                traceback_err.message.to_string(),
                file!().to_string(),
                line!(),
            )
            .with_parent(traceback_err.clone())
        } else {
            $crate::error_types::TracebackError::new(String::from(""), file!().to_string(), line!())
                .with_extra_data(json!({
                    "error": err_string
                }))
        }
    }};
    ($e:expr, $msg:expr) => {{
        use $crate::serde_json::json;
        let err_string = $e.to_string();
        let mut boxed: Box<dyn std::any::Any> = Box::new($e);
        if let Some(traceback_err) = boxed.downcast_mut::<TracebackError>() {
            traceback_err.is_handled = true;
            $crate::error_types::TracebackError::new(
                $msg.to_string(),
                file!().to_string(),
                line!(),
            )
            .with_parent(traceback_err.clone())
        } else {
            $crate::error_types::TracebackError::new(String::from(""), file!().to_string(), line!())
                .with_extra_data(json!({
                    "error": err_string
                }))
        }
    }};
}
