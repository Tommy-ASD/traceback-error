# traceback-error

traceback-error is a Rust crate for efficient error handling and traceback functionality. It simplifies error tracking, serialization, and handling in your Rust projects, making it easier to manage and diagnose errors.

## Features

Efficient Error Handling: traceback-error provides a structured way to handle errors, allowing you to track and manage them effectively.

Traceback Functionality: Easily create error traces that include detailed information about the error, its location, and any relevant context.

Serialization: Serialize errors to JSON for storage or debugging purposes.

## Installation

Add this crate to your Cargo.toml:

```toml
[dependencies]
traceback-error = "0.1.2"
```

## Usage

```rs
use traceback_error::TracebackError;

fn main() {
    // Create a new TracebackError
    let error = TracebackError::new("An error occurred".to_string(), file!().to_string(), line!());

    // Add extra data
    let error_with_data = error
        .with_extra_data(json!({
            "details": "Additional information about the error"
        }))
        .with_project("MyProject")
        .with_computer_name("MyComputer")
        .with_username("MyUser");

    // Handle the error
    if let Err(err) = do_something_that_might_fail() {
        // Handle the error here
        // You can also log or serialize it
        println!("Error: {}", err);
        // Or continue tracing
        traceback!(e, "The thing that might fail failed");
    }
}

fn do_something_that_might_fail() -> Result<(), TracebackError> {
    // Your code here
    // Use the traceback! macro to create and handle errors
    Ok(())
}
```

## Documentation

Detailed documentation is currently work in progress

## Contributing

Contributions are welcome! Feel free to open issues or pull requests on the GitHub repository.
This project is still in very early development, and proper contribution guidelines have not yet been established

## License

This crate is dual-licensed under the [MIT License](LICENSE-MIT) and the [Apache License, Version 2.0](LICENSE-APACHE-2.0). You may choose either of these licenses when using this crate. See the [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE-2.0](LICENSE-APACHE-2.0) files for the full text of the licenses.