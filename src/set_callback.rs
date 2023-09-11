use std::future::Future;
use std::pin::Pin;

use crate::{TracebackError, TRACEBACK_ERROR_CALLBACK};

pub trait TracebackCallback {
    fn call(&self, error: TracebackError);
}

// Define a trait that represents a function returning a Future
pub trait TracebackCallbackAsync {
    fn call(&self, error: TracebackError) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>;
}

pub enum TracebackCallbackType {
    Async(Box<dyn TracebackCallbackAsync + Send + Sync>),
    Sync(Box<dyn TracebackCallback + Send + Sync>),
}

pub fn set_traceback_callback(callback: TracebackCallbackType) {
    unsafe {
        TRACEBACK_ERROR_CALLBACK = Some(callback);
    }
}

pub fn reset_traceback_callback() {
    unsafe {
        TRACEBACK_ERROR_CALLBACK = None;
    }
}

/// Sets a custom traceback callback for error handling in a Rust program.
///
/// This macro allows you to define and set a custom traceback callback function,
/// which will be called when a TracebackError goes out of scope.
/// The traceback callback provides a way to customize how error information is
/// handled and reported.
///
/// # Usage
///
/// To use this macro, provide the name of the callback function you want to use
/// as the custom traceback callback. This function should take an argument of
/// type `traceback_error::TracebackError`. The macro generates a unique
/// struct and function to wrap your callback and sets it as the traceback
/// callback using `traceback_error::set_traceback_callback`.
///
/// # Example
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
/// ```rust
/// // The same is possible with asynchronous functions
/// async fn my_traceback_callback(error: traceback_error::TracebackError) {
///     // Custom error handling logic here
///     println!("Async custom traceback callback called: {:?}", error);
/// }
///
/// // But you have to specify that it is asynchronous
/// traceback_error::set_traceback!(async my_traceback_callback);
/// ```
#[macro_export]
macro_rules! set_traceback {
    ($callback:ident) => {
        $crate::paste::unique_paste! {
            // Generate a unique identifier for the struct
            #[allow(non_camel_case_types)]
            mod [<_private_ $callback _ TempStruct>] {
                pub struct [<$callback _ TempStruct>];

                impl $crate::set_callback::TracebackCallback for [<$callback _ TempStruct>] {
                    fn call(&self, error: $crate::TracebackError) {
                        super::$callback(error)
                    }
                }
            }

            // Expose the generated struct through a function
            pub fn [<$callback _ temp_struct>]() -> [<_private_ $callback _ TempStruct>]::[<$callback _ TempStruct>] {
                [<_private_ $callback _ TempStruct>]::[<$callback _ TempStruct>]
            }

            // Call the macro to set the traceback callback
            $crate::set_callback::set_traceback_callback($crate::set_callback::TracebackCallbackType::Sync(Box::new([<$callback _ temp_struct>]())));
        }
    };
    (async $callback:ident) => {
        $crate::paste::unique_paste! {
            // Generate a unique identifier for the struct
            #[allow(non_camel_case_types)]
            mod [<_private_ $callback _ TempStruct>] {
                pub struct [<$callback _ TempStruct>];

                impl $crate::set_callback::TracebackCallbackAsync for [<$callback _ TempStruct>] {
                    fn call(
                        &self,
                        error: $crate::TracebackError,
                    ) -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = ()> + std::marker::Send + std::marker::Sync>,
                    > {
                        Box::pin(super::$callback(error))
                    }
                }
            }

            // Expose the generated struct through a function
            pub fn [<$callback _ temp_struct>]() -> [<_private_ $callback _ TempStruct>]::[<$callback _ TempStruct>] {
                [<_private_ $callback _ TempStruct>]::[<$callback _ TempStruct>]
            }

            // Call the macro to set the traceback callback
            $crate::set_callback::set_traceback_callback($crate::set_callback::TracebackCallbackType::Async(Box::new([<$callback _ temp_struct>]())));
        }
    };
}
