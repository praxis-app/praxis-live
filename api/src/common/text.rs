pub(crate) fn sanitize_text(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    let mut characters = value.trim().chars().peekable();
    while let Some(character) = characters.next() {
        if character != '<' {
            sanitized.push(character);
            continue;
        }

        let mut possible_tag = String::from("<");
        let mut found_tag_end = false;
        for character in characters.by_ref() {
            possible_tag.push(character);
            if character == '>' {
                found_tag_end = true;
                break;
            }
        }

        if !found_tag_end {
            sanitized.push_str(&possible_tag);
        }
    }
    sanitized
}
