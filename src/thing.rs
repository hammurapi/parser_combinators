// Define the macro
macro_rules! pub_extern {
    // Turn a function definition into a pub extern with the matching arch specific calling convention:
    // * on x86: stdcall
    // * on all other platforms: the platform specific default calling convention (c-call)
    //
    // Match a function declaration and an attribute
    ($(#[$attr:meta])* fn $name:ident($($param:ident: $param_type:ty),*) -> $return_type:ty $body:block) => {

        // stdcall on target_arch == "x86"
        #[cfg(target_arch = "x86")]
        $(#[$attr])*
        #[no_mangle]
        pub extern "stdcall" fn $name($($param: $param_type),*) -> $return_type {
            $body
        }

        // default calling calling convention on target_arch != "x86"
        #[cfg(not(target_arch = "x86"))]
        $(#[$attr])*
        #[no_mangle]
        pub extern fn $name($($param: $param_type),*) -> $return_type {
            $body
        }
    };
}

// Example usage of the macro
pub_extern! {
    /// This is a wrapped function
    fn wrapped_function(x: i32, y: i32) -> i32 {
        x + y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapped_function() {
        // Call the wrapped function
        let result = wrapped_function(10, 20);
        println!("Result: {}", result);
    }
}
