pub mod qh;
pub mod v1;
mod session;

pub use session::{
    DeveloperSession, 
    RequestType
};

#[macro_export]
macro_rules! developer_endpoint {
    ($endpoint:expr) => {
        format!("https://developerservices2.apple.com/services{}", $endpoint)
    };
}

// Apple apis restrict certain characters in app names
pub fn strip_invalid_chars(str: &str) -> String {
    const INVALID_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '.'];

    str.chars()
        .filter(|c| 
            c.is_ascii() 
            && !c.is_control() 
            && !INVALID_CHARS.contains(c)
        )
        .collect()
}
