#[test]
fn test_resource_uri_parsing() {
    let uri = "document:///home/user/doc.txt";
    assert!(uri.starts_with("document://"));
    let raw_path = uri.trim_start_matches("document://");
    assert_eq!(raw_path, "/home/user/doc.txt");
}

#[test]
fn test_resource_uri_with_encoded_chars() {
    let uri = "document:///home/user/my%20file.txt";
    let raw_path = uri.trim_start_matches("document://");
    let decoded = urlencoding_decode(raw_path).unwrap();
    assert_eq!(decoded, "/home/user/my file.txt");
}

#[test]
fn test_resource_uri_with_plus() {
    let uri = "document:///home/user/file+name.txt";
    let raw_path = uri.trim_start_matches("document://");
    let decoded = urlencoding_decode(raw_path).unwrap();
    assert_eq!(decoded, "/home/user/file name.txt");
}

#[test]
fn test_resource_uri_invalid_scheme() {
    let uri = "http://example.com/file.txt";
    assert!(!uri.starts_with("document://"));
}

#[test]
fn test_resource_uri_empty() {
    let uri = "document://";
    let raw_path = uri.trim_start_matches("document://");
    assert_eq!(raw_path, "");
}

fn urlencoding_decode(s: &str) -> Result<String, ()> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next().ok_or(())?;
            let lo = chars.next().ok_or(())?;
            let byte = u8::from_str_radix(&format!("{}{}", hi, lo), 16).map_err(|_| ())?;
            result.push(byte as char);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    Ok(result)
}
