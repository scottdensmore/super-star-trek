//! Go `path/filepath` helpers with Unix (`/`) semantics: `Clean`, `Join`,
//! `Dir`, `Base`, `Ext`.

/// Go `filepath.Clean`.
pub fn clean(path: &str) -> String {
    if path.is_empty() {
        return ".".into();
    }
    let rooted = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if let Some(last) = parts.last() {
                    if *last != ".." {
                        parts.pop();
                        continue;
                    }
                }
                if !rooted {
                    parts.push("..");
                }
            }
            s => parts.push(s),
        }
    }
    let body = parts.join("/");
    if rooted {
        format!("/{}", body)
    } else if body.is_empty() {
        ".".into()
    } else {
        body
    }
}

/// Go `filepath.Join`.
pub fn join(elems: &[&str]) -> String {
    let non_empty: Vec<&str> = elems.iter().copied().filter(|e| !e.is_empty()).collect();
    if non_empty.is_empty() {
        return String::new();
    }
    clean(&non_empty.join("/"))
}

/// Go `filepath.Base`.
pub fn base(path: &str) -> String {
    if path.is_empty() {
        return ".".into();
    }
    let p = path.trim_end_matches('/');
    if p.is_empty() {
        return "/".into();
    }
    match p.rfind('/') {
        Some(i) => p[i + 1..].to_string(),
        None => p.to_string(),
    }
}

/// Go `filepath.Dir`.
pub fn dir(path: &str) -> String {
    let i = path.rfind('/').map(|i| i + 1).unwrap_or(0);
    clean(&path[..i])
}

/// Go `filepath.Ext`.
pub fn ext(path: &str) -> String {
    for (i, c) in path.char_indices().rev() {
        if c == '/' {
            break;
        }
        if c == '.' {
            return path[i..].to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(dir("a/b/c.TRK"), "a/b");
        assert_eq!(dir("c.TRK"), ".");
        assert_eq!(dir("/c"), "/");
        assert_eq!(base("a/b/c.TRK"), "c.TRK");
        assert_eq!(base("a/b/"), "b");
        assert_eq!(ext("x.tar.gz"), ".gz");
        assert_eq!(join(&[".", "x.TRK"]), "x.TRK");
        assert_eq!(join(&["/home/u", ".config"]), "/home/u/.config");
    }
}
